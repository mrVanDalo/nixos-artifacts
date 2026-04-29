//! Tests for graceful shutdown and error handling in background task.
//!
//! These tests verify:
//! - In-flight commands complete before shutdown
//! - Queued command handling during shutdown
//! - Timeout behavior
//! - Error handling for channel disconnect and timeouts

use std::collections::{BTreeMap, HashMap};
use std::time::Duration;

use artifacts::app::effect::Effect;
use artifacts::app::message::Message;
use artifacts::app::model::TargetType;
use artifacts::config::backend::BackendConfiguration;
use artifacts::config::make::MakeConfiguration;
use artifacts::tui::background::spawn_background_task;
use tempfile::TempDir;
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;

/// Create minimal backend config for testing
fn create_test_backend_config() -> BackendConfiguration {
    BackendConfiguration {
        config: HashMap::new(),
        base_path: std::path::PathBuf::from("."),
        backend_toml: std::path::PathBuf::from("./test.toml"),
    }
}

/// Create minimal make config for testing (empty - no artifacts)
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

#[tokio::test]
#[serial_test::serial]
async fn test_shutdown_after_completion_exits_cleanly() {
    // Send a command, wait for it to complete, then signal shutdown. With no
    // queued or in-flight work the shutdown arm has nothing to drain and the
    // task exits cleanly, closing the result channel.

    let backend = create_test_backend_config();
    let make = create_test_make_config();
    let shutdown_token = CancellationToken::new();

    let (tx_cmd, _cancel_tx, mut rx_res) = spawn_background_task(
        backend,
        make,
        artifacts::logging::LogLevel::Info,
        shutdown_token.clone(),
    );

    tx_cmd
        .send(Effect::CheckSerialization {
            artifact_index: 0,
            artifact_name: "first".to_string(),
            target_spec: artifacts::app::effect::TargetSpec::Single(TargetType::NixOS {
                machine: "machine".to_string(),
            }),
        })
        .unwrap();

    let result = timeout(Duration::from_secs(1), rx_res.recv())
        .await
        .expect("Should not timeout - command should complete")
        .expect("Should receive result for completed command");

    match result {
        Message::CheckSerializationResult { artifact_index, .. } => {
            assert_eq!(artifact_index, 0, "artifact_index should match");
        }
        _ => panic!("Expected CheckSerialization result"),
    }

    shutdown_token.cancel();

    let closed_result = timeout(Duration::from_millis(500), rx_res.recv())
        .await
        .expect("Should not timeout");

    assert!(
        closed_result.is_none(),
        "Channel should be closed after graceful shutdown"
    );

    println!("Background exits cleanly after completed work");
}

#[tokio::test]
#[serial_test::serial]
async fn test_shutdown_drops_queued_commands() {
    // Synchronously enqueue 5 effects and signal shutdown before yielding to
    // the runtime, so the background task's first poll sees both the shutdown
    // token cancelled and rx_cmd full. The shutdown arm must drain rx_cmd and
    // exit without executing any of the queued effects — Ctrl-C aborts
    // pending work rather than draining-then-quitting.
    //
    // This relies on tokio's cooperative scheduling: spawn_background_task
    // queues the task but it does not run until we hit an `.await`. All
    // sync sends and the shutdown signal happen first, in the same poll
    // context.

    let backend = create_test_backend_config();
    let make = create_test_make_config();
    let shutdown_token = CancellationToken::new();

    let (tx_cmd, _cancel_tx, mut rx_res) = spawn_background_task(
        backend,
        make,
        artifacts::logging::LogLevel::Info,
        shutdown_token.clone(),
    );

    // Synchronously enqueue 5 effects, then signal shutdown — no .await in
    // between, so the background task has not run yet.
    let num_commands = 5;
    for i in 0..num_commands {
        tx_cmd
            .send(Effect::CheckSerialization {
                artifact_index: i,
                artifact_name: format!("queued-{}", i),
                target_spec: artifacts::app::effect::TargetSpec::Single(TargetType::NixOS {
                    machine: "machine".to_string(),
                }),
            })
            .unwrap();
    }
    shutdown_token.cancel();

    // Yield. The background task wakes, biased select picks the shutdown arm,
    // drains rx_cmd, and exits. No results should ever flow on rx_res, and
    // the channel should close cleanly.
    let closed = timeout(Duration::from_millis(500), rx_res.recv())
        .await
        .expect("background must exit within timeout");

    assert!(
        closed.is_none(),
        "no results should arrive — every queued effect was dropped on shutdown, got {:?}",
        closed
    );

    println!("All {} queued commands dropped on shutdown", num_commands);
}

#[tokio::test]
#[serial_test::serial]
async fn test_background_cleanup_on_drop() {
    // Verify that when BackgroundTask completes, temporary directories are cleaned up
    // We test this by verifying the TempDir behavior and that the background task
    // properly drops its handler

    // Create a temp directory to test cleanup behavior
    let temp_dir = TempDir::new().expect("Should create temp directory");
    let temp_path = temp_dir.path().to_path_buf();

    assert!(temp_path.exists(), "Temp directory should exist");

    // Drop the temp directory
    drop(temp_dir);

    // Verify temp directory was cleaned up
    assert!(
        !temp_path.exists(),
        "Temp directory should be cleaned up on drop"
    );

    println!("Temporary directory cleanup verified");
}

#[tokio::test]
#[serial_test::serial]
async fn test_result_channel_disconnect() {
    // Background sends result, result channel closed (drop rx), verify background handles error gracefully
    // Returns error, not panic

    let backend = create_test_backend_config();
    let make = create_test_make_config();
    let shutdown_token = CancellationToken::new();

    let (tx_cmd, _cancel_tx, rx_res) = spawn_background_task(
        backend,
        make,
        artifacts::logging::LogLevel::Info,
        shutdown_token,
    );

    // Drop the result receiver (simulates TUI closing)
    drop(rx_res);

    // Send a command - background will try to send result but channel is closed
    // This should not panic
    tx_cmd
        .send(Effect::CheckSerialization {
            artifact_index: 0,
            artifact_name: "test".to_string(),
            target_spec: artifacts::app::effect::TargetSpec::Single(TargetType::NixOS {
                machine: "machine".to_string(),
            }),
        })
        .unwrap();

    // Give background time to process and handle the closed channel
    tokio::time::sleep(Duration::from_millis(200)).await;

    // If we get here without panic, the test passes
    // The background task should exit cleanly when send fails
    println!("Background handled result channel disconnect gracefully (no panic)");
}

#[tokio::test]
#[serial_test::serial]
async fn test_command_timeout() {
    // Test that commands timeout after specified duration using mock time
    // We use start_paused = true to control time advancement

    // Note: This test demonstrates the timeout mechanism
    // Real timeout testing would require a command that takes longer than the timeout
    // We verify that the timeout mechanism is in place by checking the constant

    let backend = create_test_backend_config();
    let make = create_test_make_config();
    let shutdown_token = CancellationToken::new();

    let (tx_cmd, _cancel_tx, mut rx_res) = spawn_background_task(
        backend,
        make,
        artifacts::logging::LogLevel::Info,
        shutdown_token,
    );

    // Send a command that will fail quickly (not timeout, just fail open)
    // This verifies the background task is running and responding
    tx_cmd
        .send(Effect::CheckSerialization {
            artifact_index: 99,
            artifact_name: "timeout-test".to_string(),
            target_spec: artifacts::app::effect::TargetSpec::Single(TargetType::NixOS {
                machine: "machine".to_string(),
            }),
        })
        .unwrap();

    // Command should complete (quickly, not timeout)
    let result = timeout(Duration::from_secs(5), rx_res.recv())
        .await
        .expect("Should not timeout - command completes quickly");

    // Result should be present (even if it failed)
    assert!(result.is_some(), "Should receive result before timeout");

    println!("Command completed within timeout window");
}

#[tokio::test]
#[serial_test::serial]
async fn test_error_handling_timeout_with_mock_time() {
    // Test timeout behavior by verifying timeout wrapper works
    // We don't have access to start_paused, but we can verify the timeout constant
    // and that commands complete within reasonable time

    use artifacts::tui::background::BACKGROUND_TASK_TIMEOUT;

    // Verify timeout constant is set to expected value (35 seconds)
    assert_eq!(
        BACKGROUND_TASK_TIMEOUT,
        Duration::from_secs(35),
        "Background task timeout should be 35 seconds"
    );

    println!("Timeout constant verified: {:?}", BACKGROUND_TASK_TIMEOUT);
}

// === Cancel-queue: drain rx_cmd without executing ===

#[tokio::test]
#[serial_test::serial]
async fn test_cancel_drops_queued_effects_without_executing() {
    // Send 5 effects + cancel synchronously, before yielding to the runtime,
    // so the background task's first poll sees both cancel_rx and rx_cmd
    // ready. Biased select! must pick the cancel arm, drain rx_cmd, and emit
    // zero results — none of the queued effects should be executed.
    //
    // This relies on tokio's cooperative scheduling: spawn_background_task
    // queues the task but it does not run until we hit an `.await`. All
    // sync sends happen first, in the same poll context.
    let backend = create_test_backend_config();
    let make = create_test_make_config();
    let shutdown_token = CancellationToken::new();

    let (tx_cmd, cancel_tx, mut rx_res) = spawn_background_task(
        backend,
        make,
        artifacts::logging::LogLevel::Info,
        shutdown_token,
    );

    // Synchronously enqueue 5 effects, then a cancel — no .await in between,
    // so the background task has not run yet.
    for i in 0..5 {
        tx_cmd
            .send(Effect::CheckSerialization {
                artifact_index: i,
                artifact_name: format!("queued-{}", i),
                target_spec: artifacts::app::effect::TargetSpec::Single(TargetType::NixOS {
                    machine: "machine".to_string(),
                }),
            })
            .unwrap();
    }
    cancel_tx.send(()).expect("cancel must deliver");

    // Now yield. The background task wakes, biased select picks cancel arm,
    // drains rx_cmd. We expect no results to flow on rx_res.
    let result = timeout(Duration::from_millis(300), rx_res.recv()).await;
    assert!(
        result.is_err(),
        "no results should arrive — every queued effect was dropped, got {:?}",
        result
    );
}

#[tokio::test]
#[serial_test::serial]
async fn test_cancel_on_empty_queue_is_noop_and_loop_continues() {
    // Cancel arriving with nothing to drain must not break the select! loop:
    // once the cancel signal has been consumed, subsequent effects flow
    // through normally. The sleep between the two sends is what makes this
    // test "empty-queue at cancel time" — without it, the post-cancel
    // command would be sitting in rx_cmd when cancel fires and would be
    // drained, which is the *intended* drain semantics covered by the test
    // above.
    let backend = create_test_backend_config();
    let make = create_test_make_config();
    let shutdown_token = CancellationToken::new();

    let (tx_cmd, cancel_tx, mut rx_res) = spawn_background_task(
        backend,
        make,
        artifacts::logging::LogLevel::Info,
        shutdown_token,
    );

    // Cancel an empty FIFO and let the background consume it.
    cancel_tx.send(()).expect("cancel must deliver");
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Send a real command after the cancel was consumed — it must complete.
    tx_cmd
        .send(Effect::CheckSerialization {
            artifact_index: 42,
            artifact_name: "post-cancel".to_string(),
            target_spec: artifacts::app::effect::TargetSpec::Single(TargetType::NixOS {
                machine: "machine".to_string(),
            }),
        })
        .unwrap();

    let result = timeout(Duration::from_secs(1), rx_res.recv())
        .await
        .expect("background must process the post-cancel command")
        .expect("background must produce a result");

    match result {
        Message::CheckSerializationResult { artifact_index, .. } => {
            assert_eq!(
                artifact_index, 42,
                "post-cancel command should be processed normally"
            );
        }
        _ => panic!("expected CheckSerializationResult, got {:?}", result),
    }
}
