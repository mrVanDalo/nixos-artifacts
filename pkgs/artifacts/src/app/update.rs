use super::effect::Effect;
use super::message::{CheckOutput, GeneratorOutput, KeyEvent, Msg, SerializeOutput};
use super::model::*;
use crossterm::event::{KeyCode, KeyModifiers};

/// Compute the initial effect to run when the app starts.
/// This triggers check_serialization for all pending artifacts.
pub fn init(model: &Model) -> Effect {
    let effects: Vec<Effect> = model
        .entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| entry.status() == &ArtifactStatus::Pending)
        .map(|(i, entry)| match entry {
            ListEntry::Single(single) => Effect::CheckSerialization {
                artifact_index: i,
                artifact_name: single.artifact.name.clone(),
                target: single.target.clone(),
                target_type: single.target_type,
            },
            ListEntry::Shared(shared) => Effect::SharedCheckSerialization {
                artifact_index: i,
                artifact_name: shared.info.artifact_name.clone(),
                backend_name: shared.info.backend_name.clone(),
                nixos_targets: shared.info.nixos_targets.clone(),
                home_targets: shared.info.home_targets.clone(),
            },
        })
        .collect();
    Effect::batch(effects)
}

/// Pure state transition: (Model, Msg) -> (Model, Effect)
/// This function has NO side effects - it only computes new state.
pub fn update(model: Model, msg: Msg) -> (Model, Effect) {
    match (&model.screen, msg) {
        // === Artifact List Screen ===
        (Screen::ArtifactList, Msg::Key(key)) => update_artifact_list(model, key),

        // === Generator Selection Screen ===
        (Screen::SelectGenerator(_), Msg::Key(key)) => update_generator_selection(model, key),

        // === Prompt Screen ===
        (Screen::Prompt(_), Msg::Key(key)) => update_prompt(model, key),

        // === Generating Screen ===
        (
            Screen::Generating(_),
            Msg::GeneratorFinished {
                artifact_index,
                result,
            },
        ) => handle_generator_finished(model, artifact_index, result),
        (
            Screen::Generating(_),
            Msg::SerializeFinished {
                artifact_index,
                result,
            },
        ) => handle_serialize_finished(model, artifact_index, result),
        (
            Screen::Generating(_),
            Msg::SharedGeneratorFinished {
                artifact_index,
                result,
            },
        ) => handle_shared_generator_finished(model, artifact_index, result),
        (
            Screen::Generating(_),
            Msg::SharedSerializeFinished {
                artifact_index,
                result,
            },
        ) => handle_shared_serialize_finished(model, artifact_index, result),

        // === Check serialization results (any screen) ===
        (
            _,
            Msg::CheckSerializationResult {
                artifact_index,
                result,
                output,
            },
        ) => handle_check_result(model, artifact_index, result, output),

        // === Shared check serialization results (any screen) ===
        (
            _,
            Msg::SharedCheckSerializationResult {
                artifact_index,
                result,
                output,
            },
        ) => handle_check_result(model, artifact_index, result, output),

        // === Global ===
        (_, Msg::Quit) => (model, Effect::Quit),
        (_, Msg::Tick) => {
            let mut model = model;
            model.tick_count += 1;
            (model, Effect::None)
        }

        // Unhandled combinations
        _ => (model, Effect::None),
    }
}

fn update_artifact_list(mut model: Model, key: KeyEvent) -> (Model, Effect) {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => (model, Effect::Quit),

        KeyCode::Up | KeyCode::Char('k') => {
            if model.selected_index > 0 {
                model.selected_index -= 1;
            }
            (model, Effect::None)
        }

        KeyCode::Down | KeyCode::Char('j') => {
            if model.selected_index + 1 < model.entries.len() {
                model.selected_index += 1;
            }
            (model, Effect::None)
        }

        KeyCode::Tab => {
            model.selected_log_step = model.selected_log_step.next();
            (model, Effect::None)
        }

        KeyCode::Enter => start_generation_for_selected(model),

        _ => (model, Effect::None),
    }
}

fn start_generation_for_selected(mut model: Model) -> (Model, Effect) {
    let Some(entry) = model.entries.get(model.selected_index) else {
        return (model, Effect::None);
    };

    match entry {
        ListEntry::Single(single) => {
            let prompt_state = create_prompt_state(model.selected_index, single);

            if prompt_state.prompts.is_empty() {
                // No prompts needed, go straight to generating
                let effect = Effect::RunGenerator {
                    artifact_index: model.selected_index,
                    artifact_name: single.artifact.name.clone(),
                    target: single.target.clone(),
                    target_type: single.target_type,
                    prompts: Default::default(),
                };
                model.screen = Screen::Generating(GeneratingState {
                    artifact_index: model.selected_index,
                    artifact_name: single.artifact.name.clone(),
                    step: GenerationStep::RunningGenerator,
                    log_lines: vec![],
                });
                (model, effect)
            } else {
                model.screen = Screen::Prompt(prompt_state);
                (model, Effect::None)
            }
        }
        ListEntry::Shared(shared) => {
            // Block generation for validation errors
            if shared.info.error.is_some() {
                // Error already displayed in log panel via status
                // Return to list without starting generation
                return (model, Effect::None);
            }

            let artifact_index = model.selected_index;
            let artifact_name = shared.info.artifact_name.clone();

            // Check if there's only one unique generator
            if shared.info.generators.len() == 1 {
                // Smart selection: skip dialog, use the only generator
                let generator_path = shared.info.generators[0].path.clone();
                let files: Vec<_> = shared.info.files.keys().cloned().collect();
                let nixos_targets = shared.info.nixos_targets.clone();
                let home_targets = shared.info.home_targets.clone();
                let prompts: Vec<PromptEntry> = shared
                    .info
                    .prompts
                    .values()
                    .map(|p| PromptEntry {
                        name: p.name.clone(),
                        description: p.description.clone(),
                    })
                    .collect();

                // Store the selected generator in the shared entry
                if let Some(ListEntry::Shared(shared)) = model.entries.get_mut(artifact_index) {
                    shared.selected_generator = Some(generator_path.clone());
                }

                if prompts.is_empty() {
                    // No prompts needed, go straight to generating
                    let effect = Effect::RunSharedGenerator {
                        artifact_index,
                        artifact_name: artifact_name.clone(),
                        generator_path,
                        prompts: Default::default(),
                        nixos_targets,
                        home_targets,
                        files,
                    };
                    model.screen = Screen::Generating(GeneratingState {
                        artifact_index,
                        artifact_name: artifact_name.clone(),
                        step: GenerationStep::RunningGenerator,
                        log_lines: vec![],
                    });
                    (model, effect)
                } else {
                    // Need to collect prompts first
                    model.screen = Screen::Prompt(PromptState {
                        artifact_index,
                        artifact_name: artifact_name.clone(),
                        prompts,
                        current_prompt_index: 0,
                        input_mode: InputMode::Line,
                        buffer: String::new(),
                        collected: Default::default(),
                    });
                    (model, Effect::None)
                }
            } else {
                // Multiple generators: show selection dialog
                model.screen = Screen::SelectGenerator(SelectGeneratorState {
                    artifact_index,
                    artifact_name: artifact_name.clone(),
                    generators: shared.info.generators.clone(),
                    selected_index: 0,
                });

                // The effect is mostly informational now (screen is already set)
                let effect = Effect::ShowGeneratorSelection {
                    artifact_index,
                    artifact_name,
                };
                (model, effect)
            }
        }
    }
}

fn update_prompt(mut model: Model, key: KeyEvent) -> (Model, Effect) {
    let Screen::Prompt(ref mut state) = model.screen else {
        return (model, Effect::None);
    };

    match key.code {
        KeyCode::Esc => {
            // Cancel, go back to list
            model.screen = Screen::ArtifactList;
            (model, Effect::None)
        }

        KeyCode::Tab if state.buffer.is_empty() => {
            state.input_mode = state.input_mode.next();
            (model, Effect::None)
        }

        KeyCode::Enter => handle_prompt_enter(model),

        KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            handle_prompt_ctrl_d(model)
        }

        KeyCode::Char(c)
            if !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
        {
            let Screen::Prompt(ref mut state) = model.screen else {
                return (model, Effect::None);
            };
            state.buffer.push(c);
            (model, Effect::None)
        }

        KeyCode::Backspace => {
            let Screen::Prompt(ref mut state) = model.screen else {
                return (model, Effect::None);
            };
            state.buffer.pop();
            (model, Effect::None)
        }

        _ => (model, Effect::None),
    }
}

fn handle_prompt_enter(mut model: Model) -> (Model, Effect) {
    let Screen::Prompt(ref mut state) = model.screen else {
        return (model, Effect::None);
    };

    match state.input_mode {
        InputMode::Line | InputMode::Hidden => {
            // Submit current prompt
            let name = state.prompts[state.current_prompt_index].name.clone();
            state
                .collected
                .insert(name, std::mem::take(&mut state.buffer));
            state.current_prompt_index += 1;

            if state.is_complete() {
                finish_prompts_and_start_generation(model)
            } else {
                // Reset for next prompt
                let Screen::Prompt(ref mut state) = model.screen else {
                    return (model, Effect::None);
                };
                state.input_mode = InputMode::Line;
                (model, Effect::None)
            }
        }
        InputMode::Multiline => {
            state.buffer.push('\n');
            (model, Effect::None)
        }
    }
}

fn handle_prompt_ctrl_d(mut model: Model) -> (Model, Effect) {
    let Screen::Prompt(ref mut state) = model.screen else {
        return (model, Effect::None);
    };

    if state.input_mode == InputMode::Multiline {
        // Submit multiline input
        let name = state.prompts[state.current_prompt_index].name.clone();
        state
            .collected
            .insert(name, std::mem::take(&mut state.buffer));
        state.current_prompt_index += 1;

        if state.is_complete() {
            finish_prompts_and_start_generation(model)
        } else {
            state.input_mode = InputMode::Line;
            (model, Effect::None)
        }
    } else {
        (model, Effect::None)
    }
}

fn finish_prompts_and_start_generation(mut model: Model) -> (Model, Effect) {
    let Screen::Prompt(state) = &model.screen else {
        return (model, Effect::None);
    };

    let artifact_index = state.artifact_index;
    let prompts = state.collected.clone();
    let artifact_name = state.artifact_name.clone();

    model.screen = Screen::Generating(GeneratingState {
        artifact_index,
        artifact_name: artifact_name.clone(),
        step: GenerationStep::RunningGenerator,
        log_lines: vec![],
    });

    // Build effect based on entry type
    let effect = match &model.entries[artifact_index] {
        ListEntry::Single(single) => Effect::RunGenerator {
            artifact_index,
            artifact_name,
            target: single.target.clone(),
            target_type: single.target_type,
            prompts,
        },
        ListEntry::Shared(shared) => {
            // For shared artifacts, need to use the selected generator
            let generator_path = shared.selected_generator.clone().unwrap_or_default();
            let files: Vec<_> = shared.info.files.keys().cloned().collect();
            Effect::RunSharedGenerator {
                artifact_index,
                artifact_name,
                generator_path,
                prompts,
                nixos_targets: shared.info.nixos_targets.clone(),
                home_targets: shared.info.home_targets.clone(),
                files,
            }
        }
    };

    (model, effect)
}

fn create_prompt_state(artifact_index: usize, entry: &ArtifactEntry) -> PromptState {
    PromptState {
        artifact_index,
        artifact_name: entry.artifact.name.clone(),
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
    result: Result<bool, String>,
    output: Option<CheckOutput>,
) -> (Model, Effect) {
    if let Some(entry) = model.entries.get_mut(artifact_index) {
        // Add captured script output to logs
        if let Some(check_output) = output {
            let step_logs = entry.step_logs_mut();
            for line in &check_output.stdout_lines {
                step_logs.check.push(LogEntry {
                    level: LogLevel::Output,
                    message: line.clone(),
                });
            }
            for line in &check_output.stderr_lines {
                step_logs.check.push(LogEntry {
                    level: LogLevel::Error,
                    message: line.clone(),
                });
            }
        }

        // Add status summary
        match result {
            Ok(true) => {
                *entry.status_mut() = ArtifactStatus::NeedsGeneration;
                entry.step_logs_mut().check.push(LogEntry {
                    level: LogLevel::Info,
                    message: "Artifact needs regeneration".to_string(),
                });
            }
            Ok(false) => {
                *entry.status_mut() = ArtifactStatus::UpToDate;
                entry.step_logs_mut().check.push(LogEntry {
                    level: LogLevel::Success,
                    message: "Already up to date".to_string(),
                });
            }
            Err(e) => {
                *entry.status_mut() = ArtifactStatus::Failed {
                    error: e.clone(),
                    output: String::new(),
                    retry_available: true,
                };
                entry.step_logs_mut().check.push(LogEntry {
                    level: LogLevel::Error,
                    message: e,
                });
            }
        }
    }
    (model, Effect::None)
}

fn handle_generator_finished(
    model: Model,
    artifact_index: usize,
    result: Result<GeneratorOutput, String>,
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
    output: GeneratorOutput,
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
            message: format!("Generated {} file(s)", output.files_generated),
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
            target: single.target.clone(),
            target_type: single.target_type,
            out_dir: Default::default(),
        },
        ListEntry::Shared(_) => {
            // Shared serialization handled separately
            Effect::None
        }
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

        let output = format_step_logs(entry);

        *entry.status_mut() = ArtifactStatus::Failed {
            error: error_msg,
            output,
            retry_available: true,
        };
    }
    model.screen = Screen::ArtifactList;
    (model, Effect::None)
}

fn handle_serialize_finished(
    model: Model,
    artifact_index: usize,
    result: Result<SerializeOutput, String>,
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
    output: SerializeOutput,
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

        let output = format_step_logs(entry);

        *entry.status_mut() = ArtifactStatus::Failed {
            error: error_msg,
            output,
            retry_available: true,
        };
    }
    model.screen = Screen::ArtifactList;
    (model, Effect::None)
}

// === Generator Selection Screen ===

fn update_generator_selection(mut model: Model, key: KeyEvent) -> (Model, Effect) {
    let Screen::SelectGenerator(ref mut state) = model.screen else {
        return (model, Effect::None);
    };

    match key.code {
        KeyCode::Esc | KeyCode::Char('q') => {
            // Cancel and return to list
            model.screen = Screen::ArtifactList;
            (model, Effect::None)
        }

        KeyCode::Up | KeyCode::Char('k') => {
            if state.selected_index > 0 {
                state.selected_index -= 1;
            }
            (model, Effect::None)
        }

        KeyCode::Down | KeyCode::Char('j') => {
            if state.selected_index + 1 < state.generators.len() {
                state.selected_index += 1;
            }
            (model, Effect::None)
        }

        KeyCode::Enter => {
            // Select the current generator and proceed to prompts/generation
            let selected_path = state
                .generators
                .get(state.selected_index)
                .map(|g| g.path.clone())
                .unwrap_or_default();
            let artifact_index = state.artifact_index;

            // Store the selected generator in the shared entry
            if let Some(ListEntry::Shared(shared)) = model.entries.get_mut(artifact_index) {
                shared.selected_generator = Some(selected_path.clone());

                // Check if prompts are needed
                let prompts: Vec<PromptEntry> = shared
                    .info
                    .prompts
                    .values()
                    .map(|p| PromptEntry {
                        name: p.name.clone(),
                        description: p.description.clone(),
                    })
                    .collect();

                if prompts.is_empty() {
                    // No prompts needed, go straight to generating
                    let files: Vec<_> = shared.info.files.keys().cloned().collect();
                    let effect = Effect::RunSharedGenerator {
                        artifact_index,
                        artifact_name: shared.info.artifact_name.clone(),
                        generator_path: selected_path,
                        prompts: Default::default(),
                        nixos_targets: shared.info.nixos_targets.clone(),
                        home_targets: shared.info.home_targets.clone(),
                        files,
                    };
                    model.screen = Screen::Generating(GeneratingState {
                        artifact_index,
                        artifact_name: shared.info.artifact_name.clone(),
                        step: GenerationStep::RunningGenerator,
                        log_lines: vec![],
                    });
                    (model, effect)
                } else {
                    // Need to collect prompts first
                    model.screen = Screen::Prompt(PromptState {
                        artifact_index,
                        artifact_name: shared.info.artifact_name.clone(),
                        prompts,
                        current_prompt_index: 0,
                        input_mode: InputMode::Line,
                        buffer: String::new(),
                        collected: Default::default(),
                    });
                    (model, Effect::None)
                }
            } else {
                // Shouldn't happen, but fall back to list
                model.screen = Screen::ArtifactList;
                (model, Effect::None)
            }
        }

        _ => (model, Effect::None),
    }
}

// === Shared Artifact Handlers ===

fn handle_shared_generator_finished(
    model: Model,
    artifact_index: usize,
    result: Result<GeneratorOutput, String>,
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
    output: GeneratorOutput,
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
            message: format!("Generated {} file(s)", output.files_generated),
        });
    }

    if let Screen::Generating(ref mut state) = model.screen {
        state.step = GenerationStep::Serializing;
    }

    let effect = match &model.entries[artifact_index] {
        ListEntry::Shared(shared) => Effect::SharedSerialize {
            artifact_index,
            artifact_name: shared.info.artifact_name.clone(),
            backend_name: shared.info.backend_name.clone(),
            out_dir: Default::default(),
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

        let output = format_step_logs(entry);

        *entry.status_mut() = ArtifactStatus::Failed {
            error: error_msg,
            output,
            retry_available: true,
        };
    }
    model.screen = Screen::ArtifactList;
    (model, Effect::None)
}

fn handle_shared_serialize_finished(
    model: Model,
    artifact_index: usize,
    result: Result<SerializeOutput, String>,
) -> (Model, Effect) {
    match result {
        Ok(output) => handle_shared_serialize_success(model, artifact_index, output),
        Err(error) => handle_shared_serialize_failure(model, artifact_index, error),
    }
}

/// Handles successful shared serialization completion.
fn handle_shared_serialize_success(
    mut model: Model,
    artifact_index: usize,
    output: SerializeOutput,
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

        let output = format_step_logs(entry);

        *entry.status_mut() = ArtifactStatus::Failed {
            error: error_msg,
            output,
            retry_available: true,
        };
    }
    model.screen = Screen::ArtifactList;
    (model, Effect::None)
}

/// Formats accumulated step logs from check and generate phases for error output.
fn format_step_logs(entry: &ListEntry) -> String {
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
mod tests {
    use super::*;
    use crate::config::make::{ArtifactDef, FileDef, PromptDef};
    use std::collections::BTreeMap;

    fn make_test_artifact(name: &str, prompts: Vec<&str>) -> ArtifactDef {
        let mut prompt_map = BTreeMap::new();
        for p in prompts {
            prompt_map.insert(
                p.to_string(),
                PromptDef {
                    name: p.to_string(),
                    description: Some(format!("Enter {}", p)),
                },
            );
        }
        ArtifactDef {
            name: name.to_string(),
            shared: false,
            files: BTreeMap::from([(
                "test".to_string(),
                FileDef {
                    name: "test".to_string(),
                    path: Some("/test".to_string()),
                    owner: None,
                    group: None,
                },
            )]),
            prompts: prompt_map,
            generator: "/gen".to_string(),
            serialization: "test".to_string(),
        }
    }

    fn make_test_model() -> Model {
        let entry1 = ArtifactEntry {
            target: "machine-one".to_string(),
            target_type: TargetType::Nixos,
            artifact: make_test_artifact("ssh-key", vec!["passphrase"]),
            status: ArtifactStatus::Pending,
            step_logs: StepLogs::default(),
        };
        let entry2 = ArtifactEntry {
            target: "machine-two".to_string(),
            target_type: TargetType::Nixos,
            artifact: make_test_artifact("api-token", vec![]),
            status: ArtifactStatus::Pending,
            step_logs: StepLogs::default(),
        };

        Model {
            screen: Screen::ArtifactList,
            artifacts: vec![entry1.clone(), entry2.clone()],
            entries: vec![ListEntry::Single(entry1), ListEntry::Single(entry2)],
            selected_index: 0,
            selected_log_step: LogStep::default(),
            error: None,
            warnings: Vec::new(),
            tick_count: 0,
        }
    }

    #[test]
    fn test_navigate_down() {
        let model = make_test_model();
        let (new_model, effect) = update(model, Msg::Key(KeyEvent::char('j')));

        assert_eq!(new_model.selected_index, 1);
        assert!(effect.is_none());
    }

    #[test]
    fn test_navigate_up() {
        let mut model = make_test_model();
        model.selected_index = 1;
        let (new_model, effect) = update(model, Msg::Key(KeyEvent::char('k')));

        assert_eq!(new_model.selected_index, 0);
        assert!(effect.is_none());
    }

    #[test]
    fn test_navigate_up_at_top_stays() {
        let model = make_test_model();
        let (new_model, _) = update(model, Msg::Key(KeyEvent::char('k')));

        assert_eq!(new_model.selected_index, 0);
    }

    #[test]
    fn test_navigate_down_at_bottom_stays() {
        let mut model = make_test_model();
        model.selected_index = 1;
        let (new_model, _) = update(model, Msg::Key(KeyEvent::char('j')));

        assert_eq!(new_model.selected_index, 1);
    }

    #[test]
    fn test_quit_with_q() {
        let model = make_test_model();
        let (_, effect) = update(model, Msg::Key(KeyEvent::char('q')));

        assert!(effect.is_quit());
    }

    #[test]
    fn test_quit_with_esc() {
        let model = make_test_model();
        let (_, effect) = update(model, Msg::Key(KeyEvent::esc()));

        assert!(effect.is_quit());
    }

    #[test]
    fn test_enter_opens_prompt_screen() {
        let model = make_test_model();
        let (new_model, _) = update(model, Msg::Key(KeyEvent::enter()));

        assert!(matches!(new_model.screen, Screen::Prompt(_)));
    }

    #[test]
    fn test_enter_skips_prompt_if_no_prompts() {
        let mut model = make_test_model();
        model.selected_index = 1; // api-token has no prompts
        let (new_model, effect) = update(model, Msg::Key(KeyEvent::enter()));

        assert!(matches!(new_model.screen, Screen::Generating(_)));
        assert!(matches!(effect, Effect::RunGenerator { .. }));
    }

    #[test]
    fn test_prompt_typing() {
        let mut model = make_test_model();
        model.screen = Screen::Prompt(PromptState {
            artifact_index: 0,
            artifact_name: "test".to_string(),
            prompts: vec![PromptEntry {
                name: "pass".to_string(),
                description: None,
            }],
            current_prompt_index: 0,
            input_mode: InputMode::Line,
            buffer: String::new(),
            collected: Default::default(),
        });

        let (model, _) = update(model, Msg::Key(KeyEvent::char('h')));
        let (model, _) = update(model, Msg::Key(KeyEvent::char('i')));

        if let Screen::Prompt(state) = &model.screen {
            assert_eq!(state.buffer, "hi");
        } else {
            panic!("Expected prompt screen");
        }
    }

    #[test]
    fn test_prompt_backspace() {
        let mut model = make_test_model();
        model.screen = Screen::Prompt(PromptState {
            artifact_index: 0,
            artifact_name: "test".to_string(),
            prompts: vec![PromptEntry {
                name: "pass".to_string(),
                description: None,
            }],
            current_prompt_index: 0,
            input_mode: InputMode::Line,
            buffer: "hello".to_string(),
            collected: Default::default(),
        });

        let (model, _) = update(model, Msg::Key(KeyEvent::backspace()));

        if let Screen::Prompt(state) = &model.screen {
            assert_eq!(state.buffer, "hell");
        } else {
            panic!("Expected prompt screen");
        }
    }

    #[test]
    fn test_prompt_tab_cycles_mode_when_empty() {
        let mut model = make_test_model();
        model.screen = Screen::Prompt(PromptState {
            artifact_index: 0,
            artifact_name: "test".to_string(),
            prompts: vec![PromptEntry {
                name: "pass".to_string(),
                description: None,
            }],
            current_prompt_index: 0,
            input_mode: InputMode::Line,
            buffer: String::new(),
            collected: Default::default(),
        });

        let (model, _) = update(model, Msg::Key(KeyEvent::tab()));

        if let Screen::Prompt(state) = &model.screen {
            assert_eq!(state.input_mode, InputMode::Multiline);
        } else {
            panic!("Expected prompt screen");
        }
    }

    #[test]
    fn test_prompt_tab_does_nothing_when_buffer_has_content() {
        let mut model = make_test_model();
        model.screen = Screen::Prompt(PromptState {
            artifact_index: 0,
            artifact_name: "test".to_string(),
            prompts: vec![PromptEntry {
                name: "pass".to_string(),
                description: None,
            }],
            current_prompt_index: 0,
            input_mode: InputMode::Line,
            buffer: "some text".to_string(),
            collected: Default::default(),
        });

        let (model, _) = update(model, Msg::Key(KeyEvent::tab()));

        if let Screen::Prompt(state) = &model.screen {
            assert_eq!(state.input_mode, InputMode::Line);
        } else {
            panic!("Expected prompt screen");
        }
    }

    #[test]
    fn test_prompt_esc_returns_to_list() {
        let mut model = make_test_model();
        model.screen = Screen::Prompt(PromptState {
            artifact_index: 0,
            artifact_name: "test".to_string(),
            prompts: vec![],
            current_prompt_index: 0,
            input_mode: InputMode::Line,
            buffer: String::new(),
            collected: Default::default(),
        });

        let (model, _) = update(model, Msg::Key(KeyEvent::esc()));

        assert!(matches!(model.screen, Screen::ArtifactList));
    }

    #[test]
    fn test_tab_cycles_log_step_on_list_screen() {
        let model = make_test_model();
        assert_eq!(model.selected_log_step, LogStep::Check);

        let (model, effect) = update(model, Msg::Key(KeyEvent::tab()));
        assert_eq!(model.selected_log_step, LogStep::Generate);
        assert!(effect.is_none());

        let (model, _) = update(model, Msg::Key(KeyEvent::tab()));
        assert_eq!(model.selected_log_step, LogStep::Serialize);

        let (model, _) = update(model, Msg::Key(KeyEvent::tab()));
        assert_eq!(model.selected_log_step, LogStep::Check);
    }

    // === Async Effect Tests ===

    /// Test that Enter key on artifact with prompts returns RunGenerator effect
    #[test]
    fn test_update_returns_run_generator_effect() {
        let mut model = make_test_model();
        model.selected_index = 1; // api-token has no prompts

        let (new_model, effect) = update(model, Msg::Key(KeyEvent::enter()));

        // Should be generating screen
        assert!(
            matches!(new_model.screen, Screen::Generating(_)),
            "Should enter generating screen"
        );
        assert!(
            matches!(effect, Effect::RunGenerator { .. }),
            "Should return RunGenerator effect"
        );
    }

    /// Test that GeneratorFinished returns Serialize effect
    #[test]
    fn test_update_returns_serialize_effect() {
        use crate::app::message::GeneratorOutput;

        let mut model = make_test_model();
        model.selected_index = 0;
        model.screen = Screen::Generating(GeneratingState {
            artifact_index: 0,
            artifact_name: "ssh-key".to_string(),
            step: GenerationStep::RunningGenerator,
            log_lines: vec![],
        });

        // Simulate successful generator completion
        let result = Ok(GeneratorOutput {
            stdout_lines: vec!["Generated key".to_string()],
            stderr_lines: vec![],
            files_generated: 2,
        });

        let (new_model, effect) = update(
            model,
            Msg::GeneratorFinished {
                artifact_index: 0,
                result,
            },
        );

        // Should move to serializing step
        assert!(
            matches!(new_model.screen, Screen::Generating(_)),
            "Should stay on generating screen"
        );
        assert!(
            matches!(effect, Effect::Serialize { .. }),
            "Should return Serialize effect after generator success"
        );
    }

    /// Test that artifact needs generation returns CheckSerialization effect
    #[test]
    fn test_update_returns_check_serialization_effect() {
        // Create model with pending artifacts - init() will return CheckSerialization effects
        let model = make_test_model();

        // Verify init() returns batch of CheckSerialization effects
        let effect = init(&model);

        match &effect {
            Effect::Batch(effects) => {
                // Should have effects for each pending artifact
                assert_eq!(
                    effects.len(),
                    2,
                    "Should check serialization for both artifacts"
                );
                for e in effects {
                    assert!(
                        matches!(e, Effect::CheckSerialization { .. }),
                        "Each effect should be CheckSerialization"
                    );
                }
            }
            _ => panic!(
                "init() should return Effect::Batch with CheckSerialization for each artifact"
            ),
        }
    }

    /// Test that GeneratorFinished result updates model status correctly
    #[test]
    fn test_update_handles_async_result() {
        use crate::app::message::GeneratorOutput;

        let mut model = make_test_model();
        model.selected_index = 0;

        // Set generating state
        model.screen = Screen::Generating(GeneratingState {
            artifact_index: 0,
            artifact_name: "ssh-key".to_string(),
            step: GenerationStep::RunningGenerator,
            log_lines: vec![],
        });

        // Update first entry to Generating status
        if let Some(entry) = model.entries.get_mut(0) {
            *entry.status_mut() =
                ArtifactStatus::Generating(crate::app::model::GeneratingSubstate {
                    step: crate::app::model::GenerationStep::RunningGenerator,
                    output: String::new(),
                });
        }

        // Simulate successful completion
        let result = Ok(GeneratorOutput {
            stdout_lines: vec!["Generated successfully".to_string()],
            stderr_lines: vec![],
            files_generated: 1,
        });

        let (new_model, _effect) = update(
            model,
            Msg::GeneratorFinished {
                artifact_index: 0,
                result,
            },
        );

        // Verify model state updated
        assert!(matches!(new_model.screen, Screen::Generating(_)));
        // Verify logs were added
        if let Screen::Generating(state) = &new_model.screen {
            assert_eq!(
                state.step,
                GenerationStep::Serializing,
                "Should move to serializing step"
            );
        }
    }

    /// Test that effect batching works correctly for multiple check operations
    #[test]
    fn test_update_effect_batching() {
        let model = make_test_model();

        let effect = init(&model);

        // init() should return a batch of check serialization effects
        match effect {
            Effect::Batch(effects) => {
                assert_eq!(
                    effects.len(),
                    2,
                    "Should have one effect per pending artifact"
                );
            }
            _ => panic!("Expected Effect::Batch from init()"),
        }
    }

    /// Test that SharedCheckSerializationResult updates shared artifact status correctly
    #[test]
    fn test_shared_check_serialization_result_updates_status() {
        use crate::app::model::SharedEntry;
        use crate::config::make::SharedArtifactInfo;

        // Create a model with a shared artifact
        let mut model = make_test_model_with_shared();

        // Initial status should be Pending
        assert_eq!(model.entries[0].status(), &ArtifactStatus::Pending);

        // Simulate successful shared check result indicating generation needed
        let (new_model, effect) = update(
            model,
            Msg::SharedCheckSerializationResult {
                artifact_index: 0,
                result: Ok(true), // true = needs generation
                output: Some(CheckOutput {
                    stdout_lines: vec!["Checking shared artifact...".to_string()],
                    stderr_lines: vec![],
                }),
            },
        );

        // Status should transition to NeedsGeneration
        assert_eq!(
            new_model.entries[0].status(),
            &ArtifactStatus::NeedsGeneration,
            "Shared artifact should transition from Pending to NeedsGeneration"
        );

        // Effect should be None
        assert!(effect.is_none());
    }

    /// Test that SharedCheckSerializationResult handles up-to-date status
    #[test]
    fn test_shared_check_serialization_result_up_to_date() {
        use crate::app::model::SharedEntry;
        use crate::config::make::SharedArtifactInfo;

        let model = make_test_model_with_shared();

        // Simulate successful shared check result indicating up-to-date
        let (new_model, effect) = update(
            model,
            Msg::SharedCheckSerializationResult {
                artifact_index: 0,
                result: Ok(false), // false = up to date
                output: None,
            },
        );

        // Status should transition to UpToDate
        assert_eq!(
            new_model.entries[0].status(),
            &ArtifactStatus::UpToDate,
            "Shared artifact should transition from Pending to UpToDate"
        );
        assert!(effect.is_none());
    }

    /// Test that SharedCheckSerializationResult handles error status
    #[test]
    fn test_shared_check_serialization_result_error() {
        use crate::app::model::SharedEntry;
        use crate::config::make::SharedArtifactInfo;

        let model = make_test_model_with_shared();

        // Simulate failed shared check
        let (new_model, effect) = update(
            model,
            Msg::SharedCheckSerializationResult {
                artifact_index: 0,
                result: Err("Check script failed".to_string()),
                output: Some(CheckOutput {
                    stdout_lines: vec![],
                    stderr_lines: vec!["Error: Backend not found".to_string()],
                }),
            },
        );

        // Status should transition to Failed
        match new_model.entries[0].status() {
            ArtifactStatus::Failed { error, .. } => {
                assert!(error.contains("Check script failed"));
            }
            other => panic!("Expected Failed status, got {:?}", other),
        }
        assert!(effect.is_none());
    }

    /// Test that single generator skips selection dialog and goes to prompts
    #[test]
    fn test_single_generator_skips_dialog() {
        use crate::app::model::SharedEntry;
        use crate::config::make::{
            GeneratorInfo, GeneratorSource, PromptDef, SharedArtifactInfo, TargetType,
        };
        use std::collections::BTreeMap;

        // Create shared artifact with only one generator
        let mut prompts_map: BTreeMap<String, PromptDef> = BTreeMap::new();
        prompts_map.insert(
            "passphrase".to_string(),
            PromptDef {
                name: "passphrase".to_string(),
                description: Some("Enter passphrase".to_string()),
            },
        );

        let shared_info = SharedArtifactInfo {
            artifact_name: "shared-ssh-key".to_string(),
            backend_name: "test-backend".to_string(),
            nixos_targets: vec!["machine-one".to_string()],
            home_targets: vec![],
            generators: vec![GeneratorInfo {
                path: "/nix/store/abc123/generator.sh".to_string(),
                sources: vec![GeneratorSource {
                    target: "machine-one".to_string(),
                    target_type: TargetType::Nixos,
                }],
            }],
            prompts: prompts_map,
            files: BTreeMap::new(),
            error: None,
        };

        let shared_entry = SharedEntry {
            info: shared_info,
            status: ArtifactStatus::Pending,
            step_logs: StepLogs::default(),
            selected_generator: None,
        };

        let model = Model {
            screen: Screen::ArtifactList,
            artifacts: vec![],
            entries: vec![ListEntry::Shared(shared_entry)],
            selected_index: 0,
            selected_log_step: LogStep::default(),
            error: None,
            warnings: Vec::new(),
            tick_count: 0,
        };

        // Press Enter on shared artifact
        let (new_model, effect) = update(model, Msg::Key(KeyEvent::enter()));

        // Should go directly to Prompt screen, not SelectGenerator
        assert!(
            matches!(new_model.screen, Screen::Prompt(_)),
            "Single generator should skip to Prompt screen, got {:?}",
            new_model.screen
        );

        // Effect should be None (prompts needed)
        assert!(
            effect.is_none(),
            "Expected no effect when prompts are needed"
        );
    }

    /// Test that single generator without prompts goes directly to generating
    #[test]
    fn test_single_generator_no_prompts_goes_to_generating() {
        use crate::app::model::SharedEntry;
        use crate::config::make::{
            FileDef, GeneratorInfo, GeneratorSource, SharedArtifactInfo, TargetType,
        };
        use std::collections::BTreeMap;

        // Create shared artifact with only one generator and no prompts
        let mut files_map: BTreeMap<String, FileDef> = BTreeMap::new();
        files_map.insert(
            "key".to_string(),
            FileDef {
                name: "key".to_string(),
                path: Some("/etc/ssh/ssh_key".to_string()),
                owner: None,
                group: None,
            },
        );

        let shared_info = SharedArtifactInfo {
            artifact_name: "shared-ssh-key".to_string(),
            backend_name: "test-backend".to_string(),
            nixos_targets: vec!["machine-one".to_string()],
            home_targets: vec![],
            generators: vec![GeneratorInfo {
                path: "/nix/store/abc123/generator.sh".to_string(),
                sources: vec![GeneratorSource {
                    target: "machine-one".to_string(),
                    target_type: TargetType::Nixos,
                }],
            }],
            prompts: BTreeMap::new(), // No prompts
            files: files_map,
            error: None,
        };

        let shared_entry = SharedEntry {
            info: shared_info,
            status: ArtifactStatus::Pending,
            step_logs: StepLogs::default(),
            selected_generator: None,
        };

        let model = Model {
            screen: Screen::ArtifactList,
            artifacts: vec![],
            entries: vec![ListEntry::Shared(shared_entry)],
            selected_index: 0,
            selected_log_step: LogStep::default(),
            error: None,
            warnings: Vec::new(),
            tick_count: 0,
        };

        // Press Enter on shared artifact
        let (new_model, effect) = update(model, Msg::Key(KeyEvent::enter()));

        // Should go directly to Generating screen
        assert!(
            matches!(new_model.screen, Screen::Generating(_)),
            "Single generator without prompts should go to Generating screen, got {:?}",
            new_model.screen
        );

        // Effect should be RunSharedGenerator
        assert!(
            matches!(effect, Effect::RunSharedGenerator { .. }),
            "Expected RunSharedGenerator effect, got {:?}",
            effect
        );

        // Verify the effect contains the correct generator path and targets
        if let Effect::RunSharedGenerator {
            generator_path,
            nixos_targets,
            home_targets,
            ..
        } = effect
        {
            assert_eq!(
                generator_path, "/nix/store/abc123/generator.sh",
                "Generator path should be preserved"
            );
            assert_eq!(
                nixos_targets,
                vec!["machine-one".to_string()],
                "NixOS targets should be preserved"
            );
            assert!(
                home_targets.is_empty(),
                "Home targets should be preserved as empty"
            );
        }
    }

    /// Test that multiple generators shows selection dialog
    #[test]
    fn test_multiple_generators_shows_dialog() {
        use crate::app::model::SharedEntry;
        use crate::config::make::{GeneratorInfo, GeneratorSource, SharedArtifactInfo, TargetType};
        use std::collections::BTreeMap;

        // Create shared artifact with multiple generators
        let shared_info = SharedArtifactInfo {
            artifact_name: "shared-ssh-key".to_string(),
            backend_name: "test-backend".to_string(),
            nixos_targets: vec!["machine-one".to_string(), "machine-two".to_string()],
            home_targets: vec![],
            generators: vec![
                GeneratorInfo {
                    path: "/nix/store/abc123/gen1.sh".to_string(),
                    sources: vec![GeneratorSource {
                        target: "machine-one".to_string(),
                        target_type: TargetType::Nixos,
                    }],
                },
                GeneratorInfo {
                    path: "/nix/store/def456/gen2.sh".to_string(),
                    sources: vec![GeneratorSource {
                        target: "machine-two".to_string(),
                        target_type: TargetType::Nixos,
                    }],
                },
            ],
            prompts: BTreeMap::new(),
            files: BTreeMap::new(),
            error: None,
        };

        let shared_entry = SharedEntry {
            info: shared_info,
            status: ArtifactStatus::Pending,
            step_logs: StepLogs::default(),
            selected_generator: None,
        };

        let model = Model {
            screen: Screen::ArtifactList,
            artifacts: vec![],
            entries: vec![ListEntry::Shared(shared_entry)],
            selected_index: 0,
            selected_log_step: LogStep::default(),
            error: None,
            warnings: Vec::new(),
            tick_count: 0,
        };

        // Press Enter on shared artifact
        let (new_model, effect) = update(model, Msg::Key(KeyEvent::enter()));

        // Should show SelectGenerator screen
        assert!(
            matches!(new_model.screen, Screen::SelectGenerator(_)),
            "Multiple generators should show SelectGenerator screen, got {:?}",
            new_model.screen
        );

        // Effect should be ShowGeneratorSelection
        assert!(
            matches!(effect, Effect::ShowGeneratorSelection { .. }),
            "Expected ShowGeneratorSelection effect, got {:?}",
            effect
        );

        // Verify generators are in the screen state
        if let Screen::SelectGenerator(state) = new_model.screen {
            assert_eq!(
                state.generators.len(),
                2,
                "SelectGenerator should have both generators"
            );
            assert_eq!(state.generators[0].path, "/nix/store/abc123/gen1.sh");
            assert_eq!(state.generators[1].path, "/nix/store/def456/gen2.sh");
        }
    }

    /// Test that selected generator is stored when single generator auto-selected
    #[test]
    fn test_single_generator_stores_selected_path() {
        use crate::app::model::SharedEntry;
        use crate::config::make::{GeneratorInfo, GeneratorSource, SharedArtifactInfo, TargetType};
        use std::collections::BTreeMap;

        // Create shared artifact with one generator
        let shared_info = SharedArtifactInfo {
            artifact_name: "shared-ssh-key".to_string(),
            backend_name: "test-backend".to_string(),
            nixos_targets: vec!["machine-one".to_string()],
            home_targets: vec![],
            generators: vec![GeneratorInfo {
                path: "/nix/store/abc123/generator.sh".to_string(),
                sources: vec![GeneratorSource {
                    target: "machine-one".to_string(),
                    target_type: TargetType::Nixos,
                }],
            }],
            prompts: BTreeMap::new(), // No prompts
            files: BTreeMap::new(),
            error: None,
        };

        let shared_entry = SharedEntry {
            info: shared_info,
            status: ArtifactStatus::Pending,
            step_logs: StepLogs::default(),
            selected_generator: None,
        };

        let model = Model {
            screen: Screen::ArtifactList,
            artifacts: vec![],
            entries: vec![ListEntry::Shared(shared_entry)],
            selected_index: 0,
            selected_log_step: LogStep::default(),
            error: None,
            warnings: Vec::new(),
            tick_count: 0,
        };

        // Press Enter on shared artifact
        let (new_model, _) = update(model, Msg::Key(KeyEvent::enter()));

        // Verify the selected_generator was stored in the entry
        if let ListEntry::Shared(shared) = &new_model.entries[0] {
            assert_eq!(
                shared.selected_generator,
                Some("/nix/store/abc123/generator.sh".to_string()),
                "Generator path should be stored in selected_generator"
            );
        } else {
            panic!("Expected ListEntry::Shared");
        }
    }

    fn make_test_model_with_shared() -> Model {
        use crate::app::model::SharedEntry;
        use crate::config::make::{GeneratorInfo, GeneratorSource, SharedArtifactInfo, TargetType};
        use std::collections::BTreeMap;

        let shared_info = SharedArtifactInfo {
            artifact_name: "shared-ssh-key".to_string(),
            backend_name: "test-backend".to_string(),
            nixos_targets: vec!["machine-one".to_string(), "machine-two".to_string()],
            home_targets: vec!["alice@host".to_string()],
            generators: vec![GeneratorInfo {
                path: "/test/generator.sh".to_string(),
                sources: vec![
                    GeneratorSource {
                        target: "machine-one".to_string(),
                        target_type: TargetType::Nixos,
                    },
                    GeneratorSource {
                        target: "machine-two".to_string(),
                        target_type: TargetType::Nixos,
                    },
                ],
            }],
            prompts: BTreeMap::new(),
            files: BTreeMap::new(),
            error: None,
        };

        let shared_entry = SharedEntry {
            info: shared_info,
            status: ArtifactStatus::Pending,
            step_logs: StepLogs::default(),
            selected_generator: None,
        };

        Model {
            screen: Screen::ArtifactList,
            artifacts: vec![],
            entries: vec![ListEntry::Shared(shared_entry)],
            selected_index: 0,
            selected_log_step: LogStep::default(),
            error: None,
            warnings: Vec::new(),
            tick_count: 0,
        }
    }
}
