//! Pure state transition functions for the Elm Architecture.
//!
//! This module implements the `update` function - the core of the Elm Architecture.
//! It takes the current model and a message, and returns a new model and an effect.
//! All functions in this module are pure (no side effects), making them easy to test.
//!
//! # Structure
//!
//! Each screen has its own submodule with an update handler:
//! - [`artifact_list`]: List navigation and generation triggers
//! - [`generator_selection`]: Generator selection for shared artifacts
//! - [`confirm_regenerate`]: Regeneration confirmation dialog
//! - [`prompt`]: Prompt input and submission
//! - [`generating`]: Generation/serialization result handlers
//! - [`chronological_log`]: Log viewing and navigation
//!
//! Cross-screen handlers (check results, output streaming, quit, tick)
//! remain in this dispatcher module.

mod artifact_list;
mod chronological_log;
mod confirm_regenerate;
mod generating;
mod generator_selection;
mod init;
mod prompt;

pub use init::init;

use super::effect::Effect;
use super::message::{Message, ScriptOutput};
use super::model::*;

// Re-exported for use by tests via `super::*`
#[cfg(test)]
use super::message::KeyEvent;

/// Pure state transition: (Model, Message) -> (Model, Effect)
/// This function has NO side effects - it only computes new state.
pub fn update(model: Model, msg: Message) -> (Model, Effect) {
    match (&model.screen, msg) {
        // === Artifact List Screen ===
        (Screen::ArtifactList, Message::Key(key)) => {
            artifact_list::update_artifact_list(model, key)
        }

        // === Generator Selection Screen ===
        (Screen::SelectGenerator(_), Message::Key(key)) => {
            generator_selection::update_generator_selection(model, key)
        }

        // === Confirm Regenerate Screen ===
        (Screen::ConfirmRegenerate(_), Message::Key(key)) => {
            confirm_regenerate::update_confirm_regenerate(model, key)
        }

        // === Prompt Screen ===
        (Screen::Prompt(_), Message::Key(key)) => prompt::update_prompt(model, key),

        // === Generating Screen ===
        (
            Screen::Generating(_),
            Message::GeneratorFinished {
                artifact_index,
                result,
            },
        ) => generating::handle_generator_finished(model, artifact_index, result),
        (
            Screen::Generating(_),
            Message::SerializeFinished {
                artifact_index,
                result,
            },
        ) => generating::handle_serialize_finished(model, artifact_index, result),
        (
            Screen::Generating(_),
            Message::SharedGeneratorFinished {
                artifact_index,
                result,
            },
        ) => generating::handle_shared_generator_finished(model, artifact_index, result),
        (
            Screen::Generating(_),
            Message::SharedSerializeFinished {
                artifact_index,
                results,
            },
        ) => generating::handle_shared_serialize_finished(model, artifact_index, results),

        // === Chronological Log Screen ===
        (Screen::ChronologicalLog(_), Message::Key(key)) => {
            chronological_log::update_chronological_log(model, key)
        }

        // === Check serialization results (any screen) ===
        (
            _,
            Message::CheckSerializationResult {
                artifact_index,
                status,
                result,
            },
        ) => handle_check_result(model, artifact_index, status, result),

        // === Shared check serialization results (any screen) ===
        (
            _,
            Message::SharedCheckSerializationResult {
                artifact_index,
                statuses,
                outputs,
            },
        ) => handle_shared_check_result(model, artifact_index, statuses, outputs),

        // === Streaming output (any screen) ===
        (
            _,
            Message::OutputLine {
                artifact_index,
                stream,
                content,
            },
        ) => handle_output_line(model, artifact_index, stream, content),

        // === Global ===
        (_, Message::Quit) => (model, Effect::Quit),
        (_, Message::Tick) => {
            let mut model = model;
            model.tick_count += 1;
            (model, Effect::None)
        }

        // Unhandled combinations
        _ => (model, Effect::None),
    }
}

pub(crate) fn create_prompt_state(artifact_index: usize, entry: &ArtifactEntry) -> PromptState {
    PromptState {
        artifact_index,
        artifact_name: entry.artifact.name.clone(),
        description: entry.artifact.description.clone(),
        prompts: entry
            .artifact
            .prompts
            .values()
            .map(|p| PromptEntry {
                name: p.name.clone(),
                description: p.description.clone(),
            })
            .collect(),
        current_prompt_index: 0,
        input_mode: InputMode::Line,
        buffer: String::new(),
        collected: Default::default(),
    }
}

fn handle_check_result(
    mut model: Model,
    artifact_index: usize,
    status: ArtifactStatus,
    result: Result<ScriptOutput, String>,
) -> (Model, Effect) {
    let Some(entry) = model.entries.get_mut(artifact_index) else {
        return (model, Effect::None);
    };
    match &result {
        Ok(output) => {
            // Add captured script output to logs using helper methods
            if !output.stdout_lines.is_empty() || !output.stderr_lines.is_empty() {
                entry
                    .step_logs_mut()
                    .append_stdout(LogStep::Check, &output.stdout_lines);
                entry
                    .step_logs_mut()
                    .append_stderr(LogStep::Check, &output.stderr_lines);
            }

            // Set status from check result
            *entry.status_mut() = status.clone();

            // Add status summary log entry
            let (level, message) = match status {
                ArtifactStatus::NeedsGeneration => {
                    (LogLevel::Info, "Artifact needs regeneration".to_string())
                }
                ArtifactStatus::UpToDate => (LogLevel::Success, "Already up to date".to_string()),
                _ => (LogLevel::Info, "Unknown status".to_string()),
            };
            entry
                .step_logs_mut()
                .check
                .push(LogEntry { level, message });
        }
        Err(e) => {
            let artifact_error = ArtifactError::IoError { context: e.clone() };
            entry.step_logs_mut().check.push(LogEntry {
                level: LogLevel::Error,
                message: e.clone(),
            });
            *entry.status_mut() = ArtifactStatus::Failed {
                error: artifact_error,
                output: String::new(),
            };
        }
    }
    (model, Effect::None)
}

// === Helper for generation flow ===

#[allow(clippy::too_many_lines)]
pub(crate) fn start_generation_for_selected_internal(
    mut model: Model,
    artifact_index: usize,
) -> (Model, Effect) {
    // Extract all needed data before any mutable borrow
    // Complex tuple type needed to extract data before mutable borrow
    #[allow(clippy::type_complexity)]
    let entry_data: Option<(
        bool,
        Option<String>,
        String,
        Option<String>,
        Vec<crate::config::make::PromptDef>,
        Vec<String>,
        Vec<String>,
        Vec<crate::config::make::GeneratorInfo>,
    )> = {
        let Some(entry) = model.entries.get(artifact_index) else {
            return (model, Effect::None);
        };
        match entry {
            ListEntry::Single(single) => {
                let _prompt_state = create_prompt_state(artifact_index, single);
                let artifact_name = single.artifact.name.clone();
                let exists_before = matches!(single.status, ArtifactStatus::UpToDate);
                // Store what we need for the branches
                Some((
                    exists_before,
                    None,
                    artifact_name,
                    None,
                    vec![],
                    vec![],
                    vec![],
                    vec![],
                ))
            }
            ListEntry::Shared(shared) => {
                if shared.info.error.is_some() {
                    return (model, Effect::None);
                }
                let artifact_name = shared.info.artifact_name.clone();
                let exists_before = matches!(shared.status, ArtifactStatus::UpToDate);
                let description = shared.info.description.clone();
                let generator_path = shared.info.generators.first().map(|g| g.path.clone());
                let prompts: Vec<_> = shared.info.prompts.values().cloned().collect();
                let nixos_targets = shared.info.nixos_targets.clone();
                let home_targets = shared.info.home_targets.clone();
                let generators = shared.info.generators.clone();
                Some((
                    exists_before,
                    generator_path,
                    artifact_name,
                    description,
                    prompts,
                    nixos_targets,
                    home_targets,
                    generators,
                ))
            }
        }
    };

    let Some((
        exists_before,
        generator_path,
        artifact_name,
        description,
        prompts,
        nixos_targets,
        home_targets,
        generators,
    )) = entry_data
    else {
        return (model, Effect::None);
    };

    // Now we can mutate model without borrowing entry
    let Some(entry) = model.entries.get(artifact_index) else {
        return (model, Effect::None);
    };

    match entry {
        ListEntry::Single(single) => {
            let prompt_state = create_prompt_state(artifact_index, single);
            let artifact_name = single.artifact.name.clone();
            let target_type = single.target_type.clone();

            if prompt_state.prompts.is_empty() {
                let effect = Effect::RunGenerator {
                    artifact_index,
                    artifact_name: artifact_name.clone(),
                    target_type,
                    prompts: Default::default(),
                };
                model.screen = Screen::Generating(GeneratingState {
                    artifact_index,
                    artifact_name,
                    step: GenerationStep::RunningGenerator,
                    log_lines: vec![],
                    exists: exists_before,
                });
                (model, effect)
            } else {
                model.screen = Screen::Prompt(prompt_state);
                (model, Effect::None)
            }
        }
        ListEntry::Shared(_) => {
            if generators.len() == 1 {
                if let Some(generator_path) = generator_path
                    && let Some(ListEntry::Shared(shared)) = model.entries.get_mut(artifact_index)
                {
                    shared.selected_generator = Some(generator_path);
                }

                if prompts.is_empty() {
                    let effect = Effect::RunSharedGenerator {
                        artifact_index,
                        artifact_name: artifact_name.clone(),
                        prompts: Default::default(),
                    };
                    model.screen = Screen::Generating(GeneratingState {
                        artifact_index,
                        artifact_name,
                        step: GenerationStep::RunningGenerator,
                        log_lines: vec![],
                        exists: exists_before,
                    });
                    (model, effect)
                } else {
                    model.screen = Screen::Prompt(PromptState {
                        artifact_index,
                        artifact_name,
                        description,
                        prompts: prompts
                            .iter()
                            .map(|p| PromptEntry {
                                name: p.name.clone(),
                                description: p.description.clone(),
                            })
                            .collect(),
                        current_prompt_index: 0,
                        input_mode: InputMode::Line,
                        buffer: String::new(),
                        collected: Default::default(),
                    });
                    (model, Effect::None)
                }
            } else {
                model.screen = Screen::SelectGenerator(SelectGeneratorState {
                    artifact_index,
                    artifact_name: artifact_name.clone(),
                    description,
                    generators,
                    selected_index: 0,
                    prompts,
                    nixos_targets,
                    home_targets,
                });
                (model, Effect::None)
            }
        }
    }
}

// === Shared Artifact Handlers ===

fn handle_shared_check_result(
    mut model: Model,
    artifact_index: usize,
    statuses: Vec<ArtifactStatus>,
    outputs: Vec<ScriptOutput>,
) -> (Model, Effect) {
    // Aggregate results: needs_generation if any needs it, exists if any is UpToDate
    let any_needs_gen = statuses
        .iter()
        .any(|s| matches!(s, ArtifactStatus::NeedsGeneration));

    let Some(entry) = model.entries.get_mut(artifact_index) else {
        return (model, Effect::None);
    };
    // Add captured script output to logs using first output if available
    if let Some(check_output) = outputs.first()
        && (!check_output.stdout_lines.is_empty() || !check_output.stderr_lines.is_empty())
    {
        entry
            .step_logs_mut()
            .append_stdout(LogStep::Check, &check_output.stdout_lines);
        entry
            .step_logs_mut()
            .append_stderr(LogStep::Check, &check_output.stderr_lines);
    }

    // Set status based on aggregated result
    if any_needs_gen {
        *entry.status_mut() = ArtifactStatus::NeedsGeneration;
        entry.step_logs_mut().check.push(LogEntry {
            level: LogLevel::Info,
            message: "Shared artifact needs regeneration".to_string(),
        });
    } else {
        *entry.status_mut() = ArtifactStatus::UpToDate;
        entry.step_logs_mut().check.push(LogEntry {
            level: LogLevel::Success,
            message: "Already up to date".to_string(),
        });
    }
    (model, Effect::None)
}

/// Handles streaming output line received during script execution.
fn handle_output_line(
    mut model: Model,
    artifact_index: usize,
    stream: crate::app::model::OutputStream,
    content: String,
) -> (Model, Effect) {
    let Some(entry) = model.entries.get_mut(artifact_index) else {
        return (model, Effect::None);
    };
    // Determine log level from stream type
    let level = match stream {
        crate::app::model::OutputStream::Stdout => LogLevel::Output,
        crate::app::model::OutputStream::Stderr => LogLevel::Error,
    };

    // Determine current step based on status
    // Use the model's selected_log_step to determine where to append
    // This ensures streaming output goes to the correct step being viewed
    let step = model.selected_log_step;

    // Append to appropriate step logs
    entry.step_logs_mut().get_mut(step).push(LogEntry {
        level,
        message: content,
    });
    (model, Effect::None)
}

/// Formats accumulated step logs from check and generate phases for error output.
pub(crate) fn format_step_logs(entry: &ListEntry) -> String {
    let mut output = String::new();
    for log in &entry.step_logs().check {
        output.push_str(&format!("[check] {}\n", log.message));
    }
    for log in &entry.step_logs().generate {
        output.push_str(&format!("[generate] {}\n", log.message));
    }
    output
}

#[cfg(test)]
mod tests;
