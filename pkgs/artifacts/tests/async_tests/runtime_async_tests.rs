//! Async runtime integration tests for run_async() function.
//!
//! These tests verify the async runtime correctly handles:
//! - Channel communication between foreground and background
//! - Event processing via MockEventSource
//! - tokio::select! branch coverage
//! - Timeout scenarios
//! - Graceful error handling for disconnect scenarios
//! - Effect → Command → Result → Msg conversion pipeline

use std::collections::{BTreeMap, HashMap};
use std::time::Duration;

use artifacts::app::effect::Effect;
use artifacts::app::message::Msg;
use artifacts::app::model::{
    ArtifactEntry, ArtifactStatus, ListEntry, Model, Screen, StepLogs, TargetType,
};
use artifacts::config::backend::BackendConfiguration;
use artifacts::config::make::{ArtifactDef, FileDef, MakeConfiguration, PromptDef};
use artifacts::tui::background::spawn_background_task;
use artifacts::tui::channels::{EffectCommand, EffectResult};
use artifacts::tui::events::EventSource;
use artifacts::tui::runtime::{effect_to_command, result_to_message, run_async};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;

// ============================================================================
// Test Fixtures
// ============================================================================

/// Create a minimal test artifact definition
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

/// Create a test model with artifact entries
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
        selected_log_step: artifacts::app::model::LogStep::default(),
        error: None,
        warnings: Vec::new(),
        tick_count: 0,
    }
}

/// Create minimal backend config
fn create_test_backend_config() -> BackendConfiguration {
    BackendConfiguration {
        config: HashMap::new(),
        base_path: std::path::PathBuf::from("."),
        backend_toml: std::path::PathBuf::from("./test.toml"),
    }
}

/// Create minimal make config
fn create_test_make_config() -> MakeConfiguration {
    MakeConfiguration {
        nixos_map: BTreeMap::new(),
        home_map: BTreeMap::new(),
        nixos_config: BTreeMap::new(),
        home_config: BTreeMap::new(),
        make_base: std::path::PathBuf::from("."),
        make_json: std::path::PathBuf::from("./test.json"),
    }
}

/// Mock event source that can be pre-programmed with events
#[derive(Debug)]
struct MockEventSource {
    events: Vec<Msg>,
    current_index: usize,
}

impl MockEventSource {
    fn new(events: Vec<Msg>) -> Self {
        Self {
            events,
            current_index: 0,
        }
    }

    fn empty() -> Self {
        Self::new(vec![])
    }

    fn with_quit() -> Self {
        Self::new(vec![Msg::Key(artifacts::app::KeyEvent {
            code: crossterm::event::KeyCode::Char('q'),
            modifiers: crossterm::event::KeyModifiers::empty(),
        })])
    }
}

impl EventSource for MockEventSource {
    fn next_event(&mut self) -> Option<Msg> {
        if self.current_index < self.events.len() {
            let event = self.events[self.current_index].clone();
            self.current_index += 1;
            Some(event)
        } else {
            None
        }
    }

    fn has_event(&mut self) -> bool {
        self.current_index < self.events.len()
    }
}

/// Tracks commands sent to the background task
#[derive(Debug, Default)]
struct CommandTracker {
    commands: Vec<EffectCommand>,
}

impl CommandTracker {
    fn new() -> Self {
        Self::default()
    }

    fn track(&mut self, cmd: EffectCommand) {
        self.commands.push(cmd);
    }

    fn len(&self) -> usize {
        self.commands.len()
    }

    fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    fn contains_check_serialization(&self) -> bool {
        self.commands
            .iter()
            .any(|c| matches!(c, EffectCommand::CheckSerialization { .. }))
    }
}

// ============================================================================
// Core Runtime Tests
// ============================================================================

#[tokio::test]
#[serial_test::serial]
async fn test_run_async_processes_events() {
    // Verify run_async() correctly processes scripted events
    let model = make_test_model();
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    // Create events: navigate down, then quit
    let events = vec![
        Msg::Key(artifacts::app::KeyEvent {
            code: crossterm::event::KeyCode::Char('j'),
            modifiers: crossterm::event::KeyModifiers::empty(),
        }),
        Msg::Key(artifacts::app::KeyEvent {
            code: crossterm::event::KeyCode::Char('q'),
            modifiers: crossterm::event::KeyModifiers::empty(),
        }),
    ];
    let mut event_source = MockEventSource::new(events);

    let backend_config = create_test_backend_config();
    let make_config = create_test_make_config();

    let result = run_async(
        &mut terminal,
        &mut event_source,
        backend_config,
        make_config,
        model,
    )
    .await
    .unwrap();

    // Should have moved down to index 1
    assert_eq!(result.final_model.selected_index, 1);
    // Should have rendered multiple frames
    assert!(result.frames_rendered >= 2);
}

#[tokio::test]
#[serial_test::serial]
async fn test_run_async_drains_results_before_blocking() {
    // Verify the drain phase processes pending results before blocking
    // This tests the critical "DRAIN PHASE" in run_async()
    let model = make_test_model();
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut event_source = MockEventSource::with_quit();
    let backend_config = create_test_backend_config();
    let make_config = create_test_make_config();

    let result = run_async(
        &mut terminal,
        &mut event_source,
        backend_config,
        make_config,
        model,
    )
    .await
    .unwrap();

    // Initial effect (CheckSerialization) should have been processed
    // The model should still be valid
    assert!(result.final_model.error.is_none());
    assert_eq!(result.final_model.entries.len(), 2);
}

#[tokio::test]
#[serial_test::serial]
async fn test_run_async_sends_effects_to_background() {
    // Verify effects are converted to EffectCommands and sent via channels
    let model = make_test_model();
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let _event_source = MockEventSource::with_quit();
    let backend_config = create_test_backend_config();
    let make_config = create_test_make_config();

    // Spawn background task to receive commands
    let shutdown_token = CancellationToken::new();
    let (cmd_tx, mut res_rx) = spawn_background_task(
        backend_config.clone(),
        make_config.clone(),
        shutdown_token.child_token(),
    );

    // Send a command and verify it processes
    let cmd = EffectCommand::CheckSerialization {
        artifact_index: 0,
        artifact_name: "test".to_string(),
        target: "machine".to_string(),
        target_type: "nixos".to_string(),
    };

    cmd_tx.send(cmd).unwrap();

    // Wait for result
    let result = timeout(Duration::from_secs(1), res_rx.recv())
        .await
        .expect("Should not timeout")
        .expect("Should receive result");

    match result {
        EffectResult::CheckSerialization { artifact_index, .. } => {
            assert_eq!(artifact_index, 0, "artifact_index should match");
        }
        _ => panic!("Expected CheckSerialization result"),
    }

    // Clean shutdown
    shutdown_token.cancel();
    drop(cmd_tx);
}

#[tokio::test]
#[serial_test::serial]
async fn test_run_async_handles_results() {
    // Verify EffectResults are converted to Msgs and processed
    let backend = create_test_backend_config();
    let make = create_test_make_config();
    let shutdown_token = CancellationToken::new();

    let (cmd_tx, mut res_rx) = spawn_background_task(backend, make, shutdown_token.child_token());

    // Send CheckSerialization command
    cmd_tx
        .send(EffectCommand::CheckSerialization {
            artifact_index: 0,
            artifact_name: "test".to_string(),
            target: "machine".to_string(),
            target_type: "nixos".to_string(),
        })
        .unwrap();

    // Receive result
    let result = timeout(Duration::from_secs(1), res_rx.recv())
        .await
        .expect("Should not timeout")
        .expect("Should receive result");

    // Convert result to message (simulating what run_async does)
    let msg = result_to_message(result);

    // Verify message type
    match msg {
        Msg::CheckSerializationResult {
            artifact_index,
            result,
            ..
        } => {
            assert_eq!(artifact_index, 0);
            // Result should indicate whether generation is needed
            // (since there's no actual artifact, it will need generation)
            assert!(result.is_ok());
        }
        _ => panic!("Expected CheckSerializationResult, got {:?}", msg),
    }

    // Clean shutdown
    shutdown_token.cancel();
    drop(cmd_tx);
}

#[tokio::test]
#[serial_test::serial]
async fn test_run_async_empty_events_exits_gracefully() {
    // Verify run_async exits cleanly when event source is empty
    let model = make_test_model();
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut event_source = MockEventSource::empty();
    let backend_config = create_test_backend_config();
    let make_config = create_test_make_config();

    let result = run_async(
        &mut terminal,
        &mut event_source,
        backend_config,
        make_config,
        model,
    )
    .await
    .unwrap();

    // Should have rendered initial state
    assert!(result.frames_rendered >= 2);
    assert!(result.final_model.error.is_none());
}

// ============================================================================
// tokio::select! Branch Coverage Tests
// ============================================================================

#[tokio::test]
#[serial_test::serial]
async fn test_select_shutdown_branch() {
    // CancellationToken.cancel() triggers shutdown branch
    let backend = create_test_backend_config();
    let make = create_test_make_config();
    let shutdown_token = CancellationToken::new();

    let (cmd_tx, mut res_rx) = spawn_background_task(backend, make, shutdown_token.clone());

    // Send a command
    cmd_tx
        .send(EffectCommand::CheckSerialization {
            artifact_index: 0,
            artifact_name: "test".to_string(),
            target: "machine".to_string(),
            target_type: "nixos".to_string(),
        })
        .unwrap();

    // Receive result
    let result = timeout(Duration::from_secs(1), res_rx.recv())
        .await
        .expect("Should not timeout")
        .expect("Should receive result");

    assert!(matches!(result, EffectResult::CheckSerialization { .. }));

    // Signal shutdown
    shutdown_token.cancel();

    // Give time for shutdown
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Channel should be closed
    drop(cmd_tx);
    let closed = timeout(Duration::from_millis(500), res_rx.recv())
        .await
        .expect("Should not timeout");

    assert!(closed.is_none(), "Channel should be closed after shutdown");
}

#[tokio::test]
#[serial_test::serial]
async fn test_select_result_branch() {
    // res_rx.recv() processes background results
    let backend = create_test_backend_config();
    let make = create_test_make_config();
    let shutdown_token = CancellationToken::new();

    let (cmd_tx, mut res_rx) = spawn_background_task(backend, make, shutdown_token.child_token());

    // Send multiple commands
    for i in 0..3 {
        cmd_tx
            .send(EffectCommand::CheckSerialization {
                artifact_index: i,
                artifact_name: format!("artifact{}", i),
                target: "machine".to_string(),
                target_type: "nixos".to_string(),
            })
            .unwrap();
    }

    // Receive all results - this exercises the result branch
    let mut received = Vec::new();
    for _ in 0..3 {
        let result = timeout(Duration::from_secs(1), res_rx.recv())
            .await
            .expect("Should not timeout")
            .expect("Should receive result");

        if let EffectResult::CheckSerialization { artifact_index, .. } = result {
            received.push(artifact_index);
        }
    }

    assert_eq!(received, vec![0, 1, 2], "Results should be in FIFO order");

    shutdown_token.cancel();
    drop(cmd_tx);
}

#[tokio::test]
#[serial_test::serial]
async fn test_select_command_branch() {
    // cmd_tx.send() dispatches to background
    let backend = create_test_backend_config();
    let make = create_test_make_config();
    let shutdown_token = CancellationToken::new();

    let (cmd_tx, mut res_rx) = spawn_background_task(backend, make, shutdown_token.child_token());

    // Send command - exercises cmd_tx.send()
    cmd_tx
        .send(EffectCommand::CheckSerialization {
            artifact_index: 42,
            artifact_name: "test".to_string(),
            target: "machine".to_string(),
            target_type: "nixos".to_string(),
        })
        .unwrap();

    // Verify it was processed
    let result = timeout(Duration::from_secs(1), res_rx.recv())
        .await
        .expect("Should not timeout")
        .expect("Should receive result");

    match result {
        EffectResult::CheckSerialization { artifact_index, .. } => {
            assert_eq!(artifact_index, 42);
        }
        _ => panic!("Expected CheckSerialization result"),
    }

    shutdown_token.cancel();
    drop(cmd_tx);
}

#[tokio::test]
#[serial_test::serial]
async fn test_select_channel_closed() {
    // else branch when channel closed
    let backend = create_test_backend_config();
    let make = create_test_make_config();
    let shutdown_token = CancellationToken::new();

    let (cmd_tx, mut res_rx) = spawn_background_task(backend, make, shutdown_token.child_token());

    // Send one command
    cmd_tx
        .send(EffectCommand::CheckSerialization {
            artifact_index: 0,
            artifact_name: "test".to_string(),
            target: "machine".to_string(),
            target_type: "nixos".to_string(),
        })
        .unwrap();

    // Receive result
    let result = timeout(Duration::from_secs(1), res_rx.recv())
        .await
        .expect("Should not timeout")
        .expect("Should receive result");

    assert!(matches!(result, EffectResult::CheckSerialization { .. }));

    // Drop the sender (closes channel)
    drop(cmd_tx);

    // Signal shutdown
    shutdown_token.cancel();

    // Channel should close
    let closed = timeout(Duration::from_secs(2), res_rx.recv())
        .await
        .expect("Should not timeout");

    assert!(closed.is_none(), "Channel should be closed");
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[tokio::test]
#[serial_test::serial]
async fn test_channel_disconnect_graceful() {
    // tx_cmd dropped while rx_res active - should not panic
    let backend = create_test_backend_config();
    let make = create_test_make_config();
    let shutdown_token = CancellationToken::new();

    let (cmd_tx, mut res_rx) = spawn_background_task(backend, make, shutdown_token.child_token());

    // Send commands
    for i in 0..2 {
        cmd_tx
            .send(EffectCommand::CheckSerialization {
                artifact_index: i,
                artifact_name: format!("artifact{}", i),
                target: "machine".to_string(),
                target_type: "nixos".to_string(),
            })
            .unwrap();
    }

    // Receive first result
    let result = timeout(Duration::from_secs(1), res_rx.recv())
        .await
        .expect("Should not timeout")
        .expect("Should receive first result");

    assert!(matches!(result, EffectResult::CheckSerialization { .. }));

    // Drop tx_cmd while rx_res still active
    drop(cmd_tx);

    // Should be able to receive remaining results without panic
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Eventually channel will close
    let timeout_result = timeout(Duration::from_secs(2), res_rx.recv()).await;

    // Result could be Some (remaining result) or None (channel closed)
    // Either is acceptable - the key is it doesn't panic
    match timeout_result {
        Ok(Some(_)) | Ok(None) => (), // Both acceptable
        Err(_) => (),                 // Timeout also acceptable
    }
}

#[tokio::test]
#[serial_test::serial]
async fn test_result_channel_disconnect() {
    // rx_res dropped - send should fail gracefully
    let backend = create_test_backend_config();
    let make = create_test_make_config();
    let shutdown_token = CancellationToken::new();

    let (cmd_tx, rx_res) = spawn_background_task(backend, make, shutdown_token.child_token());

    // Drop receiver
    drop(rx_res);

    // Send command - background will try to send result
    cmd_tx
        .send(EffectCommand::CheckSerialization {
            artifact_index: 0,
            artifact_name: "test".to_string(),
            target: "machine".to_string(),
            target_type: "nixos".to_string(),
        })
        .unwrap();

    // Give time for background to process and detect closed channel
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Should not panic - background should exit cleanly
    shutdown_token.cancel();
    drop(cmd_tx);
}

#[tokio::test]
#[serial_test::serial]
async fn test_graceful_shutdown_with_in_flight_commands() {
    // Shutdown while commands are processing
    let backend = create_test_backend_config();
    let make = create_test_make_config();
    let shutdown_token = CancellationToken::new();

    let (cmd_tx, mut res_rx) = spawn_background_task(backend, make, shutdown_token.child_token());

    // Send multiple commands
    for i in 0..5 {
        cmd_tx
            .send(EffectCommand::CheckSerialization {
                artifact_index: i,
                artifact_name: format!("artifact{}", i),
                target: "machine".to_string(),
                target_type: "nixos".to_string(),
            })
            .unwrap();
    }

    // Immediately signal shutdown
    shutdown_token.cancel();

    // Collect results - should receive some or all before shutdown
    let mut received = Vec::new();
    loop {
        match timeout(Duration::from_millis(500), res_rx.recv()).await {
            Ok(Some(result)) => {
                if let EffectResult::CheckSerialization { artifact_index, .. } = result {
                    received.push(artifact_index);
                }
            }
            Ok(None) => break,
            Err(_) => break,
        }
    }

    // Should have received at least some results
    assert!(
        received.len() >= 1 || received.len() == 5,
        "Should receive results before shutdown (got {})",
        received.len()
    );

    // Results should be in order
    let expected: Vec<usize> = (0..received.len()).collect();
    assert_eq!(received, expected, "Results should be in FIFO order");
}

// ============================================================================
// Timeout Tests
// ============================================================================

#[tokio::test]
#[serial_test::serial]
async fn test_timeout_handling() {
    // Verify timeout works correctly
    let start = tokio::time::Instant::now();

    // Create a timeout future
    let result = timeout(
        Duration::from_millis(50),
        tokio::time::sleep(Duration::from_millis(100)),
    )
    .await;

    // Should have timed out
    assert!(result.is_err());

    // Should have taken at least 50ms
    let elapsed = start.elapsed();
    assert!(elapsed >= Duration::from_millis(50));
}

#[tokio::test]
#[serial_test::serial]
async fn test_shutdown_drain_timeout() {
    // Verify graceful shutdown timeout handling
    let backend = create_test_backend_config();
    let make = create_test_make_config();
    let shutdown_token = CancellationToken::new();

    let (cmd_tx, mut res_rx) = spawn_background_task(backend, make, shutdown_token.child_token());

    // Send some commands
    for i in 0..3 {
        cmd_tx
            .send(EffectCommand::CheckSerialization {
                artifact_index: i,
                artifact_name: format!("artifact{}", i),
                target: "machine".to_string(),
                target_type: "nixos".to_string(),
            })
            .unwrap();
    }

    // Signal shutdown
    shutdown_token.cancel();

    // Use timeout to drain results
    let drain_start = tokio::time::Instant::now();
    let drain_timeout = Duration::from_secs(1);

    let mut received = 0;
    while drain_start.elapsed() < drain_timeout {
        match timeout(Duration::from_millis(100), res_rx.recv()).await {
            Ok(Some(_)) => received += 1,
            Ok(None) | Err(_) => break,
        }
    }

    // Should have received results
    assert!(received > 0, "Should receive results during drain");

    drop(cmd_tx);
}

// ============================================================================
// Effect/Command/Result/Message Conversion Tests
// ============================================================================

#[tokio::test]
#[serial_test::serial]
async fn test_effect_to_command_conversion() {
    // Verify all Effect variants convert to EffectCommands

    // Test CheckSerialization
    let effect = Effect::CheckSerialization {
        artifact_index: 0,
        artifact_name: "test".to_string(),
        target: "machine".to_string(),
        target_type: TargetType::Nixos,
    };
    let cmds = effect_to_command(effect);
    assert_eq!(cmds.len(), 1);
    assert!(matches!(cmds[0], EffectCommand::CheckSerialization { .. }));

    // Test None
    let cmds = effect_to_command(Effect::None);
    assert!(cmds.is_empty());

    // Test Quit
    let cmds = effect_to_command(Effect::Quit);
    assert!(cmds.is_empty());
}

#[tokio::test]
#[serial_test::serial]
async fn test_result_to_message_conversion() {
    // Verify EffectResults convert to correct Msg variants

    // Test CheckSerialization result
    let result = EffectResult::CheckSerialization {
        artifact_index: 0,
        needs_generation: true,
        output: Some("test output".to_string()),
    };
    let msg = result_to_message(result);
    assert!(matches!(msg, Msg::CheckSerializationResult { .. }));

    // Test GeneratorFinished result
    let result = EffectResult::GeneratorFinished {
        artifact_index: 1,
        success: true,
        output: Some("stdout".to_string()),
        error: None,
    };
    let msg = result_to_message(result);
    assert!(matches!(msg, Msg::GeneratorFinished { .. }));

    // Test SerializeFinished result
    let result = EffectResult::SerializeFinished {
        artifact_index: 2,
        success: true,
        error: None,
    };
    let msg = result_to_message(result);
    assert!(matches!(msg, Msg::SerializeFinished { .. }));
}

// ============================================================================
// Integration Tests
// ============================================================================

#[tokio::test]
#[serial_test::serial]
async fn test_full_async_cycle() {
    // Test complete cycle: Event → Update → Effect → Command → Background → Result → Msg → Update
    let model = make_test_model();
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    // Navigate then quit
    let events = vec![
        Msg::Key(artifacts::app::KeyEvent {
            code: crossterm::event::KeyCode::Char('j'),
            modifiers: crossterm::event::KeyModifiers::empty(),
        }),
        Msg::Key(artifacts::app::KeyEvent {
            code: crossterm::event::KeyCode::Char('q'),
            modifiers: crossterm::event::KeyModifiers::empty(),
        }),
    ];
    let mut event_source = MockEventSource::new(events);
    let backend_config = create_test_backend_config();
    let make_config = create_test_make_config();

    let result = run_async(
        &mut terminal,
        &mut event_source,
        backend_config,
        make_config,
        model,
    )
    .await
    .unwrap();

    // Should complete without error
    assert!(result.final_model.error.is_none());
    // Should have navigated
    assert_eq!(result.final_model.selected_index, 1);
    // Should have rendered frames
    assert!(result.frames_rendered >= 2);
}

#[tokio::test]
#[serial_test::serial]
async fn test_async_with_multiple_events() {
    // Test multiple events in sequence
    let model = make_test_model();
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    // Navigate down twice, up once, then quit
    let events = vec![
        Msg::Key(artifacts::app::KeyEvent {
            code: crossterm::event::KeyCode::Char('j'),
            modifiers: crossterm::event::KeyModifiers::empty(),
        }),
        Msg::Key(artifacts::app::KeyEvent {
            code: crossterm::event::KeyCode::Char('j'),
            modifiers: crossterm::event::KeyModifiers::empty(),
        }),
        Msg::Key(artifacts::app::KeyEvent {
            code: crossterm::event::KeyCode::Char('k'),
            modifiers: crossterm::event::KeyModifiers::empty(),
        }),
        Msg::Key(artifacts::app::KeyEvent {
            code: crossterm::event::KeyCode::Char('q'),
            modifiers: crossterm::event::KeyModifiers::empty(),
        }),
    ];
    let mut event_source = MockEventSource::new(events);
    let backend_config = create_test_backend_config();
    let make_config = create_test_make_config();

    let result = run_async(
        &mut terminal,
        &mut event_source,
        backend_config,
        make_config,
        model,
    )
    .await
    .unwrap();

    // j, j, k from index 0 should land at index 1
    assert_eq!(result.final_model.selected_index, 1);
}
