use super::super::effect::Effect;
use super::super::message::ScriptOutput;
use super::super::model::*;

// === Single Artifact Handlers ===

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

/// Handles successful generator completion.
fn handle_generator_success(
    mut model: Model,
    artifact_index: usize,
    output: ScriptOutput,
) -> (Model, Effect) {
    // Store logs in entry
    if let Some(entry) = model.entries.get_mut(artifact_index) {
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
    }

    // Move to serialization
    if let Screen::Generating(ref mut state) = model.screen {
        state.step = GenerationStep::Serializing;
    }

    // Build serialization effect based on entry type
    let effect = match &model.entries[artifact_index] {
        ListEntry::Single(single) => Effect::Serialize {
            artifact_index,
            artifact_name: single.artifact.name.clone(),
            target_type: single.target_type.clone(),
        },
        ListEntry::Shared(_) => Effect::None,
    };
    (model, effect)
}

/// Handles generator failure by logging and setting failed status.
fn handle_generator_failure(
    mut model: Model,
    artifact_index: usize,
    error: String,
) -> (Model, Effect) {
    if let Some(entry) = model.entries.get_mut(artifact_index) {
        let error_msg = format!(
            "Generator failed for '{}': {}",
            entry.artifact_name(),
            error
        );
        entry.step_logs_mut().generate.push(LogEntry {
            level: LogLevel::Error,
            message: error_msg.clone(),
        });

        let output = super::format_step_logs(entry);

        *entry.status_mut() = ArtifactStatus::Failed {
            error: error_msg,
            output,
            retry_available: true,
        };
    }
    model.screen = Screen::ArtifactList;
    (model, Effect::None)
}

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

/// Handles successful serialization completion.
fn handle_serialize_success(
    mut model: Model,
    artifact_index: usize,
    output: ScriptOutput,
) -> (Model, Effect) {
    if let Some(entry) = model.entries.get_mut(artifact_index) {
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
    }
    model.screen = Screen::ArtifactList;
    (model, Effect::None)
}

/// Handles serialization failure.
fn handle_serialize_failure(
    mut model: Model,
    artifact_index: usize,
    error: String,
) -> (Model, Effect) {
    if let Some(entry) = model.entries.get_mut(artifact_index) {
        let error_msg = format!(
            "Serialization failed for '{}': {}",
            entry.artifact_name(),
            error
        );
        entry.step_logs_mut().serialize.push(LogEntry {
            level: LogLevel::Error,
            message: error_msg.clone(),
        });

        let output = super::format_step_logs(entry);

        *entry.status_mut() = ArtifactStatus::Failed {
            error: error_msg,
            output,
            retry_available: true,
        };
    }
    model.screen = Screen::ArtifactList;
    (model, Effect::None)
}

// === Shared Artifact Handlers ===

pub(super) fn handle_shared_generator_finished(
    model: Model,
    artifact_index: usize,
    result: Result<ScriptOutput, String>,
) -> (Model, Effect) {
    match result {
        Ok(output) => handle_shared_generator_success(model, artifact_index, output),
        Err(error) => handle_shared_generator_failure(model, artifact_index, error),
    }
}

/// Handles successful shared generator completion.
fn handle_shared_generator_success(
    mut model: Model,
    artifact_index: usize,
    output: ScriptOutput,
) -> (Model, Effect) {
    if let Some(entry) = model.entries.get_mut(artifact_index) {
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
    }

    if let Screen::Generating(ref mut state) = model.screen {
        state.step = GenerationStep::Serializing;
    }

    let effect = match &model.entries[artifact_index] {
        ListEntry::Shared(shared) => Effect::SharedSerialize {
            artifact_index,
            artifact_name: shared.info.artifact_name.clone(),
            nixos_targets: shared.info.nixos_targets.clone(),
            home_targets: shared.info.home_targets.clone(),
        },
        _ => Effect::None,
    };

    (model, effect)
}

/// Handles shared generator failure.
fn handle_shared_generator_failure(
    mut model: Model,
    artifact_index: usize,
    error: String,
) -> (Model, Effect) {
    if let Some(entry) = model.entries.get_mut(artifact_index) {
        let error_msg = format!(
            "Generator failed for '{}': {}",
            entry.artifact_name(),
            error
        );
        entry.step_logs_mut().generate.push(LogEntry {
            level: LogLevel::Error,
            message: error_msg.clone(),
        });

        let output = super::format_step_logs(entry);

        *entry.status_mut() = ArtifactStatus::Failed {
            error: error_msg,
            output,
            retry_available: true,
        };
    }
    model.screen = Screen::ArtifactList;
    (model, Effect::None)
}

pub(super) fn handle_shared_serialize_finished(
    model: Model,
    artifact_index: usize,
    results: Vec<(String, bool, ScriptOutput)>,
) -> (Model, Effect) {
    // Check if all succeeded
    let all_success = results.iter().all(|(_, success, _)| *success);

    if all_success {
        handle_shared_serialize_success(model, artifact_index, results)
    } else {
        // Find first error
        let error_msg = results
            .iter()
            .filter(|(_, success, _)| !*success)
            .map(|(_, _, output)| {
                if output.stderr_lines.is_empty() {
                    "Serialization failed".to_string()
                } else {
                    output.stderr_lines.join("\n")
                }
            })
            .next()
            .unwrap_or_else(|| "Serialization failed".to_string());
        handle_shared_serialize_failure(model, artifact_index, error_msg)
    }
}

/// Handles successful shared serialization completion.
fn handle_shared_serialize_success(
    mut model: Model,
    artifact_index: usize,
    results: Vec<(String, bool, ScriptOutput)>,
) -> (Model, Effect) {
    if let Some(entry) = model.entries.get_mut(artifact_index) {
        let step_logs = entry.step_logs_mut();
        // Use first result's output for logs
        if let Some((_, _, output)) = results.first() {
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
        }
        step_logs.serialize.push(LogEntry {
            level: LogLevel::Success,
            message: "Serialized to backend (shared)".to_string(),
        });
        *entry.status_mut() = ArtifactStatus::UpToDate;
    }
    model.screen = Screen::ArtifactList;
    (model, Effect::None)
}

/// Handles shared serialization failure.
fn handle_shared_serialize_failure(
    mut model: Model,
    artifact_index: usize,
    error: String,
) -> (Model, Effect) {
    if let Some(entry) = model.entries.get_mut(artifact_index) {
        let error_msg = format!(
            "Shared serialization failed for '{}': {}",
            entry.artifact_name(),
            error
        );
        entry.step_logs_mut().serialize.push(LogEntry {
            level: LogLevel::Error,
            message: error_msg.clone(),
        });

        let output = super::format_step_logs(entry);

        *entry.status_mut() = ArtifactStatus::Failed {
            error: error_msg,
            output,
            retry_available: true,
        };
    }
    model.screen = Screen::ArtifactList;
    (model, Effect::None)
}
