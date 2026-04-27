use super::super::effect::{Effect, TargetSpec};
use super::super::message::ScriptOutput;
use super::super::model::*;

// === Generator Handlers (unified for single and shared) ===

pub(super) fn handle_generator_finished(
    model: Model,
    artifact_index: usize,
    result: Result<ScriptOutput, String>,
) -> (Model, Effect) {
    match result {
        Ok(output) => handle_generator_success(model, artifact_index, output),
        Err(error) => handle_generator_failure(model, artifact_index, error),
    }
}

/// Handles successful generator completion (single or shared).
fn handle_generator_success(
    mut model: Model,
    artifact_index: usize,
    output: ScriptOutput,
) -> (Model, Effect) {
    // Store logs in entry
    let Some(entry) = model.entries.get_mut(artifact_index) else {
        return (model, Effect::None);
    };
    let step_logs = entry.step_logs_mut();
    for line in &output.stdout_lines {
        step_logs.generate.push(LogEntry {
            level: LogLevel::Output,
            message: line.clone(),
        });
    }
    for line in &output.stderr_lines {
        step_logs.generate.push(LogEntry {
            level: LogLevel::Error,
            message: line.clone(),
        });
    }
    step_logs.generate.push(LogEntry {
        level: LogLevel::Success,
        message: "Generated files".to_string(),
    });

    // Move to serialization
    if let Screen::Generating(ref mut state) = model.screen {
        state.step = Step::Serialize;
    }

    // Build serialization effect based on entry type (using unified TargetSpec)
    let effect = match &model.entries[artifact_index] {
        ListEntry::Single(single) => Effect::Serialize {
            artifact_index,
            artifact_name: single.artifact.name.clone(),
            target_spec: TargetSpec::Single(single.target_type.clone()),
        },
        ListEntry::Shared(shared) => Effect::Serialize {
            artifact_index,
            artifact_name: shared.info.artifact_name.clone(),
            target_spec: TargetSpec::Multi {
                nixos_targets: shared.info.nixos_targets.clone(),
                home_targets: shared.info.home_targets.clone(),
            },
        },
    };
    (model, effect)
}

/// Handles generator failure by logging and setting failed status.
fn handle_generator_failure(
    mut model: Model,
    artifact_index: usize,
    error: String,
) -> (Model, Effect) {
    let Some(entry) = model.entries.get_mut(artifact_index) else {
        leave_generating_for(&mut model, artifact_index);
        return (model, Effect::None);
    };
    let artifact_name = entry.artifact_name().to_string();
    let error_msg = format!("Generator failed for '{}': {}", artifact_name, error);
    entry.step_logs_mut().generate.push(LogEntry {
        level: LogLevel::Error,
        message: error_msg,
    });

    let output = super::format_step_logs(entry);

    let artifact_error = ArtifactError::ScriptFailed {
        script_name: format!("Generator for '{}'", artifact_name),
        exit_code: None,
        stderr_summary: error,
    };

    *entry.status_mut() = ArtifactStatus::Failed {
        error: artifact_error,
        output,
    };

    leave_generating_for(&mut model, artifact_index);
    (model, Effect::None)
}

// === Serialize Handlers (unified for single and shared) ===

pub(super) fn handle_serialize_finished(
    model: Model,
    artifact_index: usize,
    result: Result<ScriptOutput, String>,
) -> (Model, Effect) {
    match result {
        Ok(output) => handle_serialize_success(model, artifact_index, output),
        Err(error) => handle_serialize_failure(model, artifact_index, error),
    }
}

/// Handles successful serialization completion (single or shared).
fn handle_serialize_success(
    mut model: Model,
    artifact_index: usize,
    output: ScriptOutput,
) -> (Model, Effect) {
    let Some(entry) = model.entries.get_mut(artifact_index) else {
        leave_generating_for(&mut model, artifact_index);
        return (model, Effect::None);
    };
    let step_logs = entry.step_logs_mut();
    for line in &output.stdout_lines {
        step_logs.serialize.push(LogEntry {
            level: LogLevel::Output,
            message: line.clone(),
        });
    }
    for line in &output.stderr_lines {
        step_logs.serialize.push(LogEntry {
            level: LogLevel::Error,
            message: line.clone(),
        });
    }
    step_logs.serialize.push(LogEntry {
        level: LogLevel::Success,
        message: "Serialized to backend".to_string(),
    });
    *entry.status_mut() = ArtifactStatus::UpToDate;

    leave_generating_for(&mut model, artifact_index);
    (model, Effect::None)
}

/// Handles serialization failure.
fn handle_serialize_failure(
    mut model: Model,
    artifact_index: usize,
    error: String,
) -> (Model, Effect) {
    let Some(entry) = model.entries.get_mut(artifact_index) else {
        leave_generating_for(&mut model, artifact_index);
        return (model, Effect::None);
    };
    let artifact_name = entry.artifact_name().to_string();
    let error_msg = format!("Serialization failed for '{}': {}", artifact_name, error);
    entry.step_logs_mut().serialize.push(LogEntry {
        level: LogLevel::Error,
        message: error_msg,
    });

    let output = super::format_step_logs(entry);

    let artifact_error = ArtifactError::ScriptFailed {
        script_name: format!("Serialization for '{}'", artifact_name),
        exit_code: None,
        stderr_summary: error,
    };

    *entry.status_mut() = ArtifactStatus::Failed {
        error: artifact_error,
        output,
    };

    leave_generating_for(&mut model, artifact_index);
    (model, Effect::None)
}

/// Returns the screen to the artifact list iff the user was watching this
/// artifact's `Screen::Generating`. Background results from the generate-all
/// flow arrive while the user is on the list (or any other screen), so we
/// only force the transition for the matching watched artifact.
fn leave_generating_for(model: &mut Model, artifact_index: usize) {
    if let Screen::Generating(state) = &model.screen
        && state.artifact_index == artifact_index
    {
        model.screen = Screen::ArtifactList;
    }
}
