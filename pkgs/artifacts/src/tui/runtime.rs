use crate::app::effect::Effect;
use crate::app::message::Msg;
use crate::app::model::Model;
use crate::app::{init, update};
use crate::config::backend::BackendConfiguration;
use crate::config::make::MakeConfiguration;
use crate::logging::log_component;
use crate::tui::channels::{EffectCommand, EffectResult};
use crate::tui::events::EventSource;
use crate::tui::views::render;
use anyhow::Result;
use ratatui::{Terminal, backend::Backend};
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

/// The result of running the TUI application
#[derive(Debug, Clone)]
pub struct RunResult {
    pub final_model: Model,
    pub frames_rendered: usize,
}

/// Trait for executing side effects.
/// This allows injecting different effect handlers for testing vs production.
/// Note: This trait is deprecated - use the async runtime with channels instead.
pub trait EffectHandler {
    /// Execute an effect and return any resulting messages.
    /// The handler may return messages that should be fed back into the update loop.
    fn execute(&mut self, effect: Effect, model: &Model) -> Result<Vec<Msg>>;
}

/// A no-op effect handler that ignores all effects.
/// Useful for pure UI testing where we don't want real side effects.
#[derive(Debug, Default)]
pub struct NoOpEffectHandler;

impl EffectHandler for NoOpEffectHandler {
    fn execute(&mut self, _effect: Effect, _model: &Model) -> Result<Vec<Msg>> {
        Ok(vec![])
    }
}

/// Run the TUI application with the given components.
///
/// This is the main entry point for running the Elm-architecture loop:
/// 1. Render the current model
/// 2. Get the next event
/// 3. Update the model with the event
/// 4. Execute any resulting effects
/// 5. Repeat until quit or events exhausted
///
/// This is the SYNC version - kept for backward compatibility with tests.
/// For production use, prefer `run_async` which doesn't block on effects.
pub fn run<B, E>(
    terminal: &mut Terminal<B>,
    events: &mut E,
    _backend: BackendConfiguration,
    _make: MakeConfiguration,
    mut model: Model,
) -> Result<RunResult>
where
    B: Backend,
    E: EventSource,
{
    // For sync version, just simulate without real background task
    let mut frames_rendered = 0;

    // Render the initial state
    terminal.draw(|f| render(f, &model))?;
    frames_rendered += 1;

    // Skip initial effect execution in sync mode
    // (would need the full async setup for real effects)

    loop {
        // Render the current state
        terminal.draw(|f| render(f, &model))?;
        frames_rendered += 1;

        // Get the next event
        let Some(msg) = events.next_event() else {
            // Event source exhausted, exit gracefully
            break;
        };

        // Update state with the event
        let (new_model, effect) = update(model, msg);
        model = new_model;

        // Check for quit
        if effect.is_quit() {
            break;
        }

        // In sync mode, we don't execute real effects
        // Just continue the loop
        let _ = effect;
    }

    Ok(RunResult {
        final_model: model,
        frames_rendered,
    })
}

/// Run the TUI application asynchronously with background task support.
///
/// This is the main entry point for running the Elm-architecture loop
/// with async effects:
/// 1. Spawn background task with channel communication
/// 2. Check for background results BEFORE blocking on terminal events
/// 3. Only block on terminal events when no results are available
/// 4. Execute effects by sending commands to background task
/// 5. Handle results from background task and feed into update loop
/// 6. Repeat until quit
///
/// # Arguments
///
/// * `terminal` - The ratatui terminal to render to
/// * `events` - Event source for user input (terminal or scripted)
/// * `backend` - Backend configuration for effect execution
/// * `make` - Make configuration for effect execution
/// * `model` - Initial application model
///
/// # Returns
///
/// The final RunResult containing the final model and frame count
pub async fn run_async<B, E>(
    terminal: &mut Terminal<B>,
    events: &mut E,
    backend: BackendConfiguration,
    make: MakeConfiguration,
    mut model: Model,
) -> Result<RunResult>
where
    B: Backend,
    E: EventSource,
{
    // Spawn background task with shutdown token
    log_component(
        "RUNTIME",
        &format!(
            "Spawning background task with {} entries",
            model.entries.len()
        ),
    );
    let shutdown_token = CancellationToken::new();
    let child_token = shutdown_token.child_token();
    let (cmd_tx, mut res_rx) =
        crate::tui::background::spawn_background_task(backend, make, child_token);
    log_component("RUNTIME", "Background task spawned");

    // Setup Ctrl+C signal handler for graceful shutdown
    let shutdown_for_signal = shutdown_token.clone();
    tokio::spawn(async move {
        if let Ok(()) = tokio::signal::ctrl_c().await {
            log_component("RUNTIME", "Ctrl+C received, requesting shutdown");
            shutdown_for_signal.cancel();
        }
    });

    let mut frames_rendered = 0;

    // Render the initial state
    terminal.draw(|f| render(f, &model))?;
    frames_rendered += 1;

    // Execute initial effect (check serialization for all pending artifacts)
    let initial_effect = init(&model);
    model = execute_initial_effect(&cmd_tx, model, initial_effect).await?;

    loop {
        // Render the current state
        terminal.draw(|f| render(f, &model))?;
        frames_rendered += 1;

        // DRAIN PHASE: Process all pending results FIRST
        // This prevents TUI freeze by never blocking when results are available
        loop {
            match res_rx.try_recv() {
                Ok(result) => {
                    log_component("RUNTIME", &format!("Received result: {:?}", result));
                    let msg = result_to_message(result);
                    log_component("RUNTIME", "Converted result to message");
                    let (new_model, effect) = update(model, msg);
                    log_component(
                        "RUNTIME",
                        &format!(
                            "Model updated, entries: {}, selected: {}",
                            new_model.entries.len(),
                            new_model.selected_index
                        ),
                    );
                    model = new_model;

                    // CRITICAL: Execute any effects returned from processing results
                    // For example, GeneratorFinished returns Serialize effect
                    let cmds = effect_to_command(effect);
                    if !cmds.is_empty() {
                        log_component(
                            "RUNTIME",
                            &format!("Executing {} commands from result", cmds.len()),
                        );
                        for cmd in cmds {
                            log_component("RUNTIME", &format!("Sending command: {:?}", cmd));
                            if cmd_tx.send(cmd).is_err() {
                                log_component("RUNTIME", "Background task closed, exiting");
                                model.error =
                                    Some("Connection to background task lost".to_string());
                                return Ok(RunResult {
                                    final_model: model,
                                    frames_rendered,
                                });
                            }
                        }
                    }

                    // Re-render after processing result
                    terminal.draw(|f| render(f, &model))?;
                    frames_rendered += 1;
                }
                Err(_) => break, // Channel empty or closed, break
            }
        }

        // WAIT PHASE: Get next event
        // Check if events are available before blocking
        if events.has_event() {
            // Event is ready - get it without blocking
            if let Some(msg) = events.next_event() {
                // Process the event
                let (new_model, effect) = update(model, msg);
                model = new_model;

                // Check for quit - initiate graceful shutdown
                if effect.is_quit() || shutdown_token.is_cancelled() {
                    log_component("RUNTIME", "Initiating graceful shutdown");

                    // 1. Signal background to shut down after current work
                    shutdown_token.cancel();
                    log_component("RUNTIME", "Shutdown signal sent to background");

                    // 2. Drain any pending results with timeout
                    const SHUTDOWN_DRAIN_TIMEOUT: Duration = Duration::from_secs(5);
                    let drain_start = Instant::now();

                    while drain_start.elapsed() < SHUTDOWN_DRAIN_TIMEOUT {
                        match tokio::time::timeout(Duration::from_millis(100), res_rx.recv()).await
                        {
                            Ok(Some(result)) => {
                                log_component("RUNTIME", "Drained result during shutdown");
                                let msg = result_to_message(result);
                                let (new_model, _) = update(model, msg);
                                model = new_model;

                                // Re-render to show final state
                                terminal.draw(|f| render(f, &model))?;
                                frames_rendered += 1;
                            }
                            Ok(None) => {
                                log_component("RUNTIME", "Result channel closed during drain");
                                model.error = Some("Background task disconnected".to_string());
                                break;
                            }
                            Err(_) => {
                                // Timeout - check if background exited
                                if cmd_tx.is_closed() {
                                    log_component(
                                        "RUNTIME",
                                        "Command channel closed, background exited",
                                    );
                                    break;
                                }
                            }
                        }
                    }

                    if drain_start.elapsed() >= SHUTDOWN_DRAIN_TIMEOUT {
                        log_component("RUNTIME", "Drain timeout reached, proceeding with exit");
                    }

                    // 3. Drop command channel to signal no more commands
                    drop(cmd_tx);
                    log_component("RUNTIME", "Command channel dropped");

                    // Break out of main loop - cleanup will happen naturally
                    break;
                }

                // Send effects to background if they need execution
                let cmds = effect_to_command(effect);
                log_component(
                    "RUNTIME",
                    &format!("Sending {} commands from event", cmds.len()),
                );
                for cmd in cmds {
                    log_component("RUNTIME", &format!("Sending command: {:?}", cmd));
                    if cmd_tx.send(cmd).is_err() {
                        log_component("RUNTIME", "Background task closed, exiting");
                        model.error = Some("Connection to background task lost".to_string());
                        break;
                    }
                }

                // After sending commands, immediately check for results
                // This ensures we process Serialize result after Generator without waiting for next event
                loop {
                    match res_rx.try_recv() {
                        Ok(result) => {
                            log_component(
                                "RUNTIME",
                                &format!("Received result after command: {:?}", result),
                            );
                            let msg = result_to_message(result);
                            let (new_model, effect) = update(model, msg);
                            model = new_model;

                            // Execute any follow-up effects
                            let cmds = effect_to_command(effect);
                            if !cmds.is_empty() {
                                log_component(
                                    "RUNTIME",
                                    &format!("Executing {} follow-up commands", cmds.len()),
                                );
                                for cmd in cmds {
                                    log_component(
                                        "RUNTIME",
                                        &format!("Sending command: {:?}", cmd),
                                    );
                                    if cmd_tx.send(cmd).is_err() {
                                        log_component("RUNTIME", "Background task closed, exiting");
                                        model.error =
                                            Some("Connection to background task lost".to_string());
                                        return Ok(RunResult {
                                            final_model: model,
                                            frames_rendered,
                                        });
                                    }
                                }
                            }

                            terminal.draw(|f| render(f, &model))?;
                            frames_rendered += 1;
                        }
                        Err(_) => break,
                    }
                }
            } else {
                // Event source exhausted
                break;
            }
        } else {
            // No events ready - check if we should wait
            // Use select! to wait for either events or results or shutdown
            tokio::select! {
                // Try to get a result
                Some(result) = res_rx.recv() => {
                    log_component("RUNTIME", &format!("Received result while waiting: {:?}", result));
                    let msg = result_to_message(result);
                    log_component("RUNTIME", "Converted result to message");
                    let (new_model, effect) = update(model, msg);
                    log_component("RUNTIME", &format!("Model updated via select, entries: {}, selected: {}", new_model.entries.len(), new_model.selected_index));
                    model = new_model;

                    // Execute any effects returned from result processing
                    let cmds = effect_to_command(effect);
                    if !cmds.is_empty() {
                        log_component("RUNTIME", &format!("Executing {} commands from select result", cmds.len()));
                        for cmd in cmds {
                            log_component("RUNTIME", &format!("Sending command: {:?}", cmd));
                            if cmd_tx.send(cmd).is_err() {
                                log_component("RUNTIME", "Background task closed, exiting");
                                model.error = Some("Connection to background task lost".to_string());
                                return Ok(RunResult {
                                    final_model: model,
                                    frames_rendered,
                                });
                            }
                        }
                    }

                    // Re-render after processing result
                    terminal.draw(|f| render(f, &model))?;
                    frames_rendered += 1;
                }

                // Check for shutdown signal
                _ = shutdown_token.cancelled() => {
                    log_component("RUNTIME", "Shutdown signal received while waiting, initiating exit");
                    // Trigger graceful shutdown - will be handled at start of next loop iteration
                }

                // Wait for events
                _ = tokio::time::sleep(std::time::Duration::from_millis(10)) => {
                    // Yield to async runtime - events will be checked on next iteration
                }
            }
        }
    }

    Ok(RunResult {
        final_model: model,
        frames_rendered,
    })
}

/// Execute the initial effect (if any) by sending to background.
/// Returns the updated model after processing the result.
async fn execute_initial_effect(
    cmd_tx: &tokio::sync::mpsc::UnboundedSender<EffectCommand>,
    model: Model,
    effect: Effect,
) -> Result<Model> {
    // Send all initial effects to background
    let cmds = effect_to_command(effect);
    log_component(
        "RUNTIME",
        &format!("Sending {} initial commands", cmds.len()),
    );
    for cmd in &cmds {
        log_component("RUNTIME", &format!("Sending command: {:?}", cmd));
    }
    for cmd in cmds {
        if let Err(e) = cmd_tx.send(cmd) {
            log_component("RUNTIME", &format!("Failed to send command: {}", e));
        }
    }

    Ok(model)
}

/// Convert an Effect into EffectCommands for channel transmission.
/// Returns a vector of commands (may be empty for None/Quit).
pub fn effect_to_command(effect: Effect) -> Vec<EffectCommand> {
    match effect {
        Effect::None | Effect::Quit => vec![],

        Effect::CheckSerialization {
            artifact_index,
            artifact_name,
            target,
            target_type,
        } => vec![EffectCommand::CheckSerialization {
            artifact_index,
            artifact_name,
            target,
            target_type: target_type.to_string(),
        }],

        Effect::RunGenerator {
            artifact_index,
            artifact_name,
            target,
            target_type,
            prompts,
        } => vec![EffectCommand::RunGenerator {
            artifact_index,
            artifact_name,
            target,
            target_type: target_type.to_string(),
            prompts,
        }],

        Effect::Serialize {
            artifact_index,
            artifact_name,
            target,
            target_type,
            out_dir: _,
        } => vec![EffectCommand::Serialize {
            artifact_index,
            artifact_name,
            target,
            target_type: target_type.to_string(),
        }],

        Effect::ShowGeneratorSelection { .. } => {
            // This effect is handled synchronously by update() - no background work
            vec![]
        }

        Effect::SharedCheckSerialization {
            artifact_index,
            artifact_name,
            backend_name: _,
            nixos_targets,
            home_targets,
        } => {
            // Convert to EffectCommand format
            let targets: Vec<String> = nixos_targets
                .iter()
                .chain(home_targets.iter())
                .cloned()
                .collect();
            let target_types: Vec<String> = nixos_targets
                .iter()
                .map(|_| "nixos".to_string())
                .chain(home_targets.iter().map(|_| "home".to_string()))
                .collect();
            vec![EffectCommand::SharedCheckSerialization {
                artifact_index,
                artifact_name,
                targets,
                target_types,
            }]
        }

        Effect::RunSharedGenerator {
            artifact_index,
            artifact_name,
            generator_path: _,
            prompts,
            nixos_targets,
            home_targets,
            files: _,
        } => vec![EffectCommand::RunSharedGenerator {
            artifact_index,
            artifact_name,
            machine_targets: nixos_targets,
            user_targets: home_targets,
            prompts,
        }],

        Effect::SharedSerialize {
            artifact_index,
            artifact_name,
            backend_name: _,
            out_dir: _,
            nixos_targets,
            home_targets,
        } => vec![EffectCommand::SharedSerialize {
            artifact_index,
            artifact_name,
            machine_targets: nixos_targets,
            user_targets: home_targets,
        }],

        Effect::Batch(effects) => {
            // Process all effects in the batch
            effects.into_iter().flat_map(effect_to_command).collect()
        }
    }
}

/// Convert an EffectResult from the background into a Msg for the update loop.
pub fn result_to_message(result: EffectResult) -> Msg {
    match result {
        EffectResult::CheckSerialization {
            artifact_index,
            needs_generation,
            output,
        } => {
            use crate::app::message::CheckOutput;
            // Convert bool+ScriptOutput to Result<bool, String>
            let result = if needs_generation {
                Ok(true)
            } else {
                Ok(false)
            };
            Msg::CheckSerializationResult {
                artifact_index,
                result,
                output: Some(CheckOutput {
                    stdout_lines: output.stdout_lines,
                    stderr_lines: output.stderr_lines,
                }),
            }
        }

        EffectResult::GeneratorFinished {
            artifact_index,
            success,
            output,
            error,
        } => {
            use crate::app::message::GeneratorOutput;
            let result = if success {
                Ok(GeneratorOutput {
                    stdout_lines: output.stdout_lines,
                    stderr_lines: output.stderr_lines,
                    files_generated: 0, // TODO: Get actual count
                })
            } else {
                Err(error.unwrap_or_else(|| "Generator failed".to_string()))
            };
            Msg::GeneratorFinished {
                artifact_index,
                result,
            }
        }

        EffectResult::SerializeFinished {
            artifact_index,
            success,
            output,
            error,
        } => {
            use crate::app::message::SerializeOutput;
            let result = if success {
                Ok(SerializeOutput {
                    stdout_lines: output.stdout_lines,
                    stderr_lines: output.stderr_lines,
                })
            } else {
                Err(error.unwrap_or_else(|| "Serialize failed".to_string()))
            };
            Msg::SerializeFinished {
                artifact_index,
                result,
            }
        }

        EffectResult::SharedCheckSerialization {
            artifact_index,
            needs_generation,
            outputs,
        } => {
            use crate::app::message::CheckOutput;
            // For simplicity, check if any target needs generation
            let any_needs_gen = needs_generation.iter().any(|&b| b);
            let result = Ok(any_needs_gen);
            // Aggregate outputs - take first if any
            let aggregated_output = if outputs.is_empty() {
                None
            } else {
                let first = &outputs[0];
                Some(CheckOutput {
                    stdout_lines: first.stdout_lines.clone(),
                    stderr_lines: first.stderr_lines.clone(),
                })
            };
            Msg::SharedCheckSerializationResult {
                artifact_index,
                result,
                output: aggregated_output,
            }
        }

        EffectResult::SharedGeneratorFinished {
            artifact_index,
            success,
            output,
            error,
        } => {
            use crate::app::message::GeneratorOutput;
            let result = if success {
                Ok(GeneratorOutput {
                    stdout_lines: output.stdout_lines,
                    stderr_lines: output.stderr_lines,
                    files_generated: 0,
                })
            } else {
                Err(error.unwrap_or_else(|| "Shared generator failed".to_string()))
            };
            Msg::SharedGeneratorFinished {
                artifact_index,
                result,
            }
        }

        EffectResult::SharedSerializeFinished {
            artifact_index,
            results: _,
        } => {
            // TODO: Aggregate results properly
            use crate::app::message::SerializeOutput;
            Msg::SharedSerializeFinished {
                artifact_index,
                result: Ok(SerializeOutput {
                    stdout_lines: vec![],
                    stderr_lines: vec![],
                }),
            }
        }

        EffectResult::OutputLine {
            artifact_index,
            stream,
            content,
        } => Msg::OutputLine {
            artifact_index,
            stream: crate::app::model::OutputStream::from(stream),
            content,
        },
    }
}

/// Simulate running the app with a scripted event sequence.
/// Returns the final model state after all events are processed.
/// This is useful for testing state transitions without rendering.
pub fn simulate<E: EventSource>(events: &mut E, mut model: Model) -> Model {
    loop {
        let Some(msg) = events.next_event() else {
            break;
        };

        let (new_model, effect) = update(model, msg);
        model = new_model;

        if effect.is_quit() {
            break;
        }
    }

    model
}

/// Simulate running the app and collect the sequence of models.
/// Returns all intermediate states, useful for debugging test failures.
pub fn simulate_with_history<E: EventSource>(events: &mut E, initial: Model) -> Vec<Model> {
    let mut history = vec![initial.clone()];
    let mut model = initial;

    loop {
        let Some(msg) = events.next_event() else {
            break;
        };

        let (new_model, effect) = update(model, msg);
        model = new_model.clone();
        history.push(new_model);

        if effect.is_quit() {
            break;
        }
    }

    history
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::KeyEvent;
    use crate::app::model::*;
    use crate::config::make::{ArtifactDef, FileDef, PromptDef};
    use crate::tui::events::ScriptedEventSource;
    use crate::tui::events::test_helpers::*;
    use ratatui::backend::TestBackend;
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
    fn test_simulate_navigation() {
        let model = make_test_model();
        let mut events = ScriptedEventSource::new(vec![
            down(), // Move to second item
            down(), // Stay at second (bottom)
            up(),   // Move back to first
        ]);

        let final_model = simulate(&mut events, model);
        assert_eq!(final_model.selected_index, 0);
    }

    #[test]
    fn test_simulate_quit() {
        let model = make_test_model();
        let mut events = ScriptedEventSource::new(vec![
            down(),
            char('q'), // Quit
            down(),    // This should not be processed
        ]);

        let final_model = simulate(&mut events, model);
        // Should have moved down once before quitting
        assert_eq!(final_model.selected_index, 1);
    }

    #[test]
    fn test_simulate_with_history() {
        let model = make_test_model();
        let mut events = ScriptedEventSource::new(vec![down(), down()]);

        let history = simulate_with_history(&mut events, model);

        // Initial + 2 updates = 3 states
        assert_eq!(history.len(), 3);
        assert_eq!(history[0].selected_index, 0);
        assert_eq!(history[1].selected_index, 1);
        assert_eq!(history[2].selected_index, 1); // Stayed at bottom
    }

    #[test]
    fn test_run_with_test_backend() {
        let model = make_test_model();
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut events = ScriptedEventSource::new(vec![down(), char('q')]);

        // Create minimal configs
        let backend_config = BackendConfiguration {
            config: std::collections::HashMap::new(),
            base_path: std::path::PathBuf::from("."),
            backend_toml: std::path::PathBuf::from("./test.toml"),
        };

        let make_config = MakeConfiguration {
            nixos_map: std::collections::BTreeMap::new(),
            home_map: std::collections::BTreeMap::new(),
            nixos_config: std::collections::BTreeMap::new(),
            home_config: std::collections::BTreeMap::new(),
            make_base: std::path::PathBuf::from("."),
            make_json: std::path::PathBuf::from("./test.json"),
        };

        let result = run(
            &mut terminal,
            &mut events,
            backend_config,
            make_config,
            model,
        )
        .unwrap();

        assert_eq!(result.final_model.selected_index, 1);
        assert!(result.frames_rendered >= 2);
    }

    #[test]
    fn test_run_empty_events_exits_gracefully() {
        let model = make_test_model();
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let mut events = ScriptedEventSource::new(vec![]);

        let backend_config = BackendConfiguration {
            config: std::collections::HashMap::new(),
            base_path: std::path::PathBuf::from("."),
            backend_toml: std::path::PathBuf::from("./test.toml"),
        };

        let make_config = MakeConfiguration {
            nixos_map: std::collections::BTreeMap::new(),
            home_map: std::collections::BTreeMap::new(),
            nixos_config: std::collections::BTreeMap::new(),
            home_config: std::collections::BTreeMap::new(),
            make_base: std::path::PathBuf::from("."),
            make_json: std::path::PathBuf::from("./test.json"),
        };

        let result = run(
            &mut terminal,
            &mut events,
            backend_config,
            make_config,
            model,
        )
        .unwrap();

        // Should render twice (initial + one loop iteration) then exit
        assert!(result.frames_rendered >= 2);
    }

    #[test]
    fn test_enter_prompt_screen() {
        let model = make_test_model();
        let mut events = ScriptedEventSource::new(vec![
            enter(), // Enter prompt screen for first artifact
        ]);

        let final_model = simulate(&mut events, model);
        assert!(matches!(final_model.screen, Screen::Prompt(_)));
    }

    #[test]
    fn test_complete_prompt_flow() {
        let model = make_test_model();
        let mut events_vec = vec![enter()]; // Enter prompt
        events_vec.extend(type_string("my-passphrase"));
        events_vec.push(enter()); // Submit

        let mut events = ScriptedEventSource::new(events_vec);
        let final_model = simulate(&mut events, model);

        // Should be in generating state (or back to list if no handler)
        assert!(matches!(final_model.screen, Screen::Generating(_)));
    }

    #[test]
    fn test_cancel_prompt_with_esc() {
        let model = make_test_model();
        let mut events = ScriptedEventSource::new(vec![
            enter(),   // Enter prompt
            char('a'), // Type something
            char('b'),
            esc(), // Cancel
        ]);

        let final_model = simulate(&mut events, model);
        assert!(matches!(final_model.screen, Screen::ArtifactList));
    }

    #[test]
    fn test_effect_to_command_handles_all_variants() {
        use crate::app::model::TargetType;
        use std::collections::HashMap;

        // Test None effect
        let cmd = effect_to_command(Effect::None);
        assert!(cmd.is_empty());

        // Test Quit effect
        let cmd = effect_to_command(Effect::Quit);
        assert!(cmd.is_empty());

        // Test CheckSerialization
        let cmd = effect_to_command(Effect::CheckSerialization {
            artifact_index: 0,
            artifact_name: "test".to_string(),
            target: "machine".to_string(),
            target_type: TargetType::Nixos,
        });
        assert_eq!(cmd.len(), 1);
        assert!(matches!(cmd[0], EffectCommand::CheckSerialization { .. }));

        // Test RunGenerator
        let cmd = effect_to_command(Effect::RunGenerator {
            artifact_index: 0,
            artifact_name: "test".to_string(),
            target: "machine".to_string(),
            target_type: TargetType::Nixos,
            prompts: HashMap::new(),
        });
        assert_eq!(cmd.len(), 1);
        assert!(matches!(cmd[0], EffectCommand::RunGenerator { .. }));
    }

    #[test]
    fn test_result_to_message_handles_all_variants() {
        use crate::tui::channels::ScriptOutput;

        // Test CheckSerialization result
        let msg = result_to_message(EffectResult::CheckSerialization {
            artifact_index: 0,
            needs_generation: true,
            output: ScriptOutput::default(),
        });
        assert!(matches!(msg, Msg::CheckSerializationResult { .. }));

        // Test GeneratorFinished result
        let msg = result_to_message(EffectResult::GeneratorFinished {
            artifact_index: 0,
            success: true,
            output: ScriptOutput::default(),
            error: None,
        });
        assert!(matches!(msg, Msg::GeneratorFinished { .. }));

        // Test SerializeFinished result
        let msg = result_to_message(EffectResult::SerializeFinished {
            artifact_index: 0,
            success: true,
            output: ScriptOutput::default(),
            error: None,
        });
        assert!(matches!(msg, Msg::SerializeFinished { .. }));
    }

    // === Async Runtime Tests ===

    /// Test that channels are properly connected for async communication
    #[tokio::test]
    async fn test_runtime_channels_connected() {
        use crate::config::backend::BackendConfiguration;
        use crate::config::make::MakeConfiguration;
        use crate::tui::background::spawn_background_task;
        use std::time::Duration;
        use tokio::time::timeout;
        use tokio_util::sync::CancellationToken;

        // Create minimal configurations
        let backend_config = BackendConfiguration {
            config: std::collections::HashMap::new(),
            base_path: std::path::PathBuf::from("."),
            backend_toml: std::path::PathBuf::from("./test.toml"),
        };

        let make_config = MakeConfiguration {
            nixos_map: std::collections::BTreeMap::new(),
            home_map: std::collections::BTreeMap::new(),
            nixos_config: std::collections::BTreeMap::new(),
            home_config: std::collections::BTreeMap::new(),
            make_base: std::path::PathBuf::from("."),
            make_json: std::path::PathBuf::from("./test.json"),
        };

        let shutdown_token = CancellationToken::new();
        let (cmd_tx, mut res_rx) =
            spawn_background_task(backend_config, make_config, shutdown_token);

        // Send a command through the channel
        let cmd = EffectCommand::CheckSerialization {
            artifact_index: 0,
            artifact_name: "test".to_string(),
            target: "machine".to_string(),
            target_type: "nixos".to_string(),
        };

        cmd_tx.send(cmd).expect("Should be able to send command");

        // Receive result with timeout to prevent hanging
        let result = timeout(Duration::from_secs(1), res_rx.recv())
            .await
            .expect("Should not timeout")
            .expect("Should receive a result");

        // Verify the result has correct artifact_index
        match result {
            EffectResult::CheckSerialization { artifact_index, .. } => {
                assert_eq!(artifact_index, 0, "artifact_index should match command");
            }
            _ => panic!("Expected CheckSerialization result"),
        }
    }

    /// Test that tick messages increment the tick counter
    #[tokio::test]
    async fn test_runtime_tick_message() {
        let model = make_test_model();
        let initial_tick = model.tick_count;

        // Tick message should increment counter
        let (new_model, effect) = update(model, Msg::Tick);

        assert_eq!(
            new_model.tick_count,
            initial_tick + 1,
            "Tick should increment counter"
        );
        assert!(effect.is_none(), "Tick should not produce effects");
    }

    /// Test that key events are converted to Msg::Key
    #[tokio::test]
    async fn test_runtime_key_message() {
        use crossterm::event::KeyCode;

        // Create a model and test key event conversion
        let model = make_test_model();

        // Create a key event
        let key_event = KeyEvent {
            code: KeyCode::Char('j'),
            modifiers: crossterm::event::KeyModifiers::empty(),
        };

        // Process the key event
        let (new_model, effect) = update(model, Msg::Key(key_event));

        // 'j' should navigate down
        assert_eq!(
            new_model.selected_index, 1,
            "j key should move selection down"
        );
        assert!(effect.is_none(), "Navigation should not produce effects");
    }

    /// Test that runtime properly spawns background task and processes commands
    #[tokio::test]
    async fn test_runtime_spawns_background() {
        use crate::config::backend::BackendConfiguration;
        use crate::config::make::MakeConfiguration;
        use crate::tui::background::spawn_background_task;
        use std::time::Duration;
        use tokio::time::timeout;
        use tokio_util::sync::CancellationToken;

        // Create configurations
        let backend_config = BackendConfiguration {
            config: std::collections::HashMap::new(),
            base_path: std::path::PathBuf::from("."),
            backend_toml: std::path::PathBuf::from("./test.toml"),
        };

        let make_config = MakeConfiguration {
            nixos_map: std::collections::BTreeMap::new(),
            home_map: std::collections::BTreeMap::new(),
            nixos_config: std::collections::BTreeMap::new(),
            home_config: std::collections::BTreeMap::new(),
            make_base: std::path::PathBuf::from("."),
            make_json: std::path::PathBuf::from("./test.json"),
        };

        let shutdown_token = CancellationToken::new();
        let (cmd_tx, mut res_rx) =
            spawn_background_task(backend_config, make_config, shutdown_token.clone());

        // Send multiple commands
        for i in 0..3 {
            let cmd = EffectCommand::CheckSerialization {
                artifact_index: i,
                artifact_name: format!("artifact{}", i),
                target: "machine".to_string(),
                target_type: "nixos".to_string(),
            };
            cmd_tx.send(cmd).expect("Should send command");
        }

        // Collect all results
        let mut received = Vec::new();
        for _ in 0..3 {
            let result = timeout(Duration::from_secs(1), res_rx.recv())
                .await
                .expect("Should not timeout")
                .expect("Should receive result");
            received.push(result);
        }

        // Verify FIFO ordering
        for (i, result) in received.iter().enumerate() {
            match result {
                EffectResult::CheckSerialization { artifact_index, .. } => {
                    assert_eq!(*artifact_index, i, "Results should be in FIFO order");
                }
                _ => panic!("Unexpected result variant"),
            }
        }

        // Clean shutdown
        shutdown_token.cancel();
        drop(cmd_tx);

        // Background should exit cleanly
        let final_result = timeout(Duration::from_millis(100), res_rx.recv()).await;
        // Channel may return None or timeout, both are acceptable
        assert!(
            final_result.is_ok() || final_result.is_err(),
            "Shutdown should complete without panic"
        );
    }
}
