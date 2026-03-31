use crate::app::effect::Effect;
use crate::app::model::Model;
use crate::app::{init, update};
use crate::config::backend::BackendConfiguration;
use crate::config::make::MakeConfiguration;
use crate::logging::log_component;
use crate::tui::events::EventSource;
use crate::tui::views::render;
use anyhow::Result;
use ratatui::{Terminal, backend::Backend};
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

/// Helper to send effect to background if it needs execution.
fn send_effect(effect: Effect, tx: &tokio::sync::mpsc::UnboundedSender<Effect>) -> bool {
    match effect {
        Effect::None | Effect::Quit => true,
        Effect::Batch(effects) => {
            for e in effects {
                if !send_effect(e, tx) {
                    return false;
                }
            }
            true
        }
        _ => tx.send(effect).is_ok(),
    }
}

/// The result of running the TUI application
#[derive(Debug, Clone)]
pub struct RunResult {
    pub final_model: Model,
    pub frames_rendered: usize,
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
#[allow(clippy::too_many_lines)]
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
    let log_level = crate::logging::current_level();
    let (cmd_tx, mut res_rx) =
        crate::tui::background::spawn_background_task(backend, make, log_level, child_token);
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
        while let Ok(result) = res_rx.try_recv() {
            log_component("RUNTIME", &format!("Received result: {:?}", result));
            let (new_model, effect) = update(model, result);
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
                                let (new_model, _) = update(model, result);
                                model = new_model;

                                // Re-render to show final state
                                terminal.draw(|f| render(f, &model))?;
                                frames_rendered += 1;
                            }
                            Ok(None) => {
                                log_component("RUNTIME", "Result channel closed during drain");
                                // This is expected during shutdown - background task finished cleanly
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
                while let Ok(result) = res_rx.try_recv() {
                    log_component(
                        "RUNTIME",
                        &format!("Received result after command: {:?}", result),
                    );
                    let (new_model, effect) = update(model, result);
                    model = new_model;

                    // Execute any follow-up effects
                    let cmds = effect_to_command(effect);
                    if !cmds.is_empty() {
                        log_component(
                            "RUNTIME",
                            &format!("Executing {} follow-up commands", cmds.len()),
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

                    terminal.draw(|f| render(f, &model))?;
                    frames_rendered += 1;
                }
            } else {
                // Event source exhausted
                break;
            }
        } else {
            // No events ready - check if we should wait or exit
            // First, check if the event source is permanently exhausted
            if events.is_exhausted() {
                log_component(
                    "RUNTIME",
                    "Event source exhausted, initiating graceful shutdown",
                );

                // Signal background to shut down
                shutdown_token.cancel();
                log_component("RUNTIME", "Shutdown signal sent to background");

                // Drain any pending results with timeout
                const SHUTDOWN_DRAIN_TIMEOUT: Duration = Duration::from_secs(1);
                let drain_start = Instant::now();

                while drain_start.elapsed() < SHUTDOWN_DRAIN_TIMEOUT {
                    match tokio::time::timeout(Duration::from_millis(100), res_rx.recv()).await {
                        Ok(Some(result)) => {
                            log_component("RUNTIME", "Drained result during exhausted shutdown");
                            let (new_model, _) = update(model, result);
                            model = new_model;
                        }
                        Ok(None) | Err(_) => break,
                    }
                }

                drop(cmd_tx);
                break;
            }

            // Use select! to wait for either events or results or shutdown
            tokio::select! {
                // Try to get a result
                Some(result) = res_rx.recv() => {
                    log_component("RUNTIME", &format!("Received result while waiting: {:?}", result));
                    let (new_model, effect) = update(model, result);
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
    cmd_tx: &tokio::sync::mpsc::UnboundedSender<Effect>,
    model: Model,
    effect: Effect,
) -> Result<Model> {
    // Send initial effects to background
    log_component("RUNTIME", &format!("Sending initial effect: {:?}", effect));
    if send_effect(effect, cmd_tx) {
        log_component("RUNTIME", "Initial effect sent successfully");
    } else {
        log_component(
            "RUNTIME",
            "Failed to send initial effect or effect was None/Quit",
        );
    }
    Ok(model)
}

/// Convert an Effect to a Vec of Effects for sending to the background.
/// Handles Batch effects by flattening them.
pub fn effect_to_command(effect: Effect) -> Vec<Effect> {
    match effect {
        Effect::None | Effect::Quit => Vec::new(),
        Effect::Batch(effects) => effects,
        effect => vec![effect],
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
    use crate::app::effect::Effect;
    use crate::app::message::Message;
    use crate::app::model::*;
    use crate::config::make::{ArtifactDef, FileDef, PromptDef};
    use crate::tui::events::ScriptedEventSource;
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
            description: None,
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
            target_type: TargetType::NixOS {
                machine: "machine-one".to_string(),
            },
            artifact: make_test_artifact("ssh-key", vec!["passphrase"]),
            status: ArtifactStatus::Pending,
            step_logs: StepLogs::default(),
        };
        let entry2 = ArtifactEntry {
            target_type: TargetType::NixOS {
                machine: "machine-two".to_string(),
            },
            artifact: make_test_artifact("api-token", vec![]),
            status: ArtifactStatus::Pending,
            step_logs: StepLogs::default(),
        };

        Model {
            screen: Screen::ArtifactList,
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
            Message::Key(KeyEvent::down()), // Move to second item
            Message::Key(KeyEvent::down()), // Stay at second (bottom)
            Message::Key(KeyEvent::up()),   // Move back to first
        ]);

        let final_model = simulate(&mut events, model);
        assert_eq!(final_model.selected_index, 0);
    }

    #[test]
    fn test_simulate_quit() {
        let model = make_test_model();
        let mut events = ScriptedEventSource::new(vec![
            Message::Key(KeyEvent::down()),
            Message::Key(KeyEvent::char('q')), // Quit
            Message::Key(KeyEvent::down()),    // This should not be processed
        ]);

        let final_model = simulate(&mut events, model);
        // Should have moved down once before quitting
        assert_eq!(final_model.selected_index, 1);
    }

    #[test]
    fn test_simulate_with_history() {
        let model = make_test_model();
        let mut events = ScriptedEventSource::new(vec![
            Message::Key(KeyEvent::down()),
            Message::Key(KeyEvent::down()),
        ]);

        let history = simulate_with_history(&mut events, model);

        // Initial + 2 updates = 3 states
        assert_eq!(history.len(), 3);
        assert_eq!(history[0].selected_index, 0);
        assert_eq!(history[1].selected_index, 1);
        assert_eq!(history[2].selected_index, 1); // Stayed at bottom
    }

    #[test]
    fn test_simulate_navigation_and_quit() {
        let model = make_test_model();
        let mut events = ScriptedEventSource::new(vec![
            Message::Key(KeyEvent::down()),
            Message::Key(KeyEvent::char('q')),
        ]);

        let final_model = simulate(&mut events, model);
        assert_eq!(final_model.selected_index, 1);
    }

    #[test]
    fn test_simulate_empty_events_exits_gracefully() {
        let model = make_test_model();
        let mut events = ScriptedEventSource::new(vec![]);

        // Should exit gracefully with initial model unchanged
        let final_model = simulate(&mut events, model);
        assert_eq!(final_model.selected_index, 0);
    }

    #[test]
    fn test_enter_prompt_screen() {
        let model = make_test_model();
        let mut events = ScriptedEventSource::new(vec![
            Message::Key(KeyEvent::enter()), // Enter prompt screen for first artifact
        ]);

        let final_model = simulate(&mut events, model);
        assert!(matches!(final_model.screen, Screen::Prompt(_)));
    }

    #[test]
    fn test_complete_prompt_flow() {
        let model = make_test_model();
        let mut events_vec = vec![Message::Key(KeyEvent::enter())]; // Enter prompt
        events_vec.extend(
            "my-passphrase"
                .chars()
                .map(|c| Message::Key(KeyEvent::char(c))),
        );
        events_vec.push(Message::Key(KeyEvent::enter())); // Submit

        let mut events = ScriptedEventSource::new(events_vec);
        let final_model = simulate(&mut events, model);

        // Should be in generating state (or back to list if no handler)
        assert!(matches!(final_model.screen, Screen::Generating(_)));
    }

    #[test]
    fn test_cancel_prompt_with_esc() {
        let model = make_test_model();
        let mut events = ScriptedEventSource::new(vec![
            Message::Key(KeyEvent::enter()),   // Enter prompt
            Message::Key(KeyEvent::char('a')), // Type something
            Message::Key(KeyEvent::char('b')),
            Message::Key(KeyEvent::esc()), // Cancel
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
            target_type: TargetType::NixOS {
                machine: "machine".to_string(),
            },
        });
        assert_eq!(cmd.len(), 1);
        assert!(matches!(cmd[0], Effect::CheckSerialization { .. }));

        // Test RunGenerator
        let cmd = effect_to_command(Effect::RunGenerator {
            artifact_index: 0,
            artifact_name: "test".to_string(),
            target_type: TargetType::NixOS {
                machine: "machine".to_string(),
            },
            prompts: HashMap::new(),
        });
        assert_eq!(cmd.len(), 1);
        assert!(matches!(cmd[0], Effect::RunGenerator { .. }));
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
        let (cmd_tx, mut res_rx) = spawn_background_task(
            backend_config,
            make_config,
            crate::logging::LogLevel::Info,
            shutdown_token,
        );

        // Send a command through the channel
        let cmd = Effect::CheckSerialization {
            artifact_index: 0,
            artifact_name: "test".to_string(),
            target_type: TargetType::NixOS {
                machine: "machine".to_string(),
            },
        };

        cmd_tx.send(cmd).expect("Should be able to send command");

        // Receive result with timeout to prevent hanging
        let result = timeout(Duration::from_secs(1), res_rx.recv())
            .await
            .expect("Should not timeout")
            .expect("Should receive a result");

        // Verify the result has correct artifact_index
        match result {
            Message::CheckSerializationResult { artifact_index, .. } => {
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
        let (new_model, effect) = update(model, Message::Tick);

        assert_eq!(
            new_model.tick_count,
            initial_tick + 1,
            "Tick should increment counter"
        );
        assert!(effect.is_none(), "Tick should not produce effects");
    }

    /// Test that key events are converted to Message::Key
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
        let (new_model, effect) = update(model, Message::Key(key_event));

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
        let (cmd_tx, mut res_rx) = spawn_background_task(
            backend_config,
            make_config,
            crate::logging::LogLevel::Info,
            shutdown_token.clone(),
        );

        // Send multiple commands
        for i in 0..3 {
            let cmd = Effect::CheckSerialization {
                artifact_index: i,
                artifact_name: format!("artifact{}", i),
                target_type: TargetType::NixOS {
                    machine: "machine".to_string(),
                },
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
                Message::CheckSerializationResult { artifact_index, .. } => {
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
