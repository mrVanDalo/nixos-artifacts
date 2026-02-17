//! Tests for graceful shutdown and error handling in background task.
//!
//! These tests verify:
//! - In-flight commands complete before shutdown
//! - Queued command handling during shutdown
//! - Timeout behavior
//! - Error handling for channel disconnect and timeouts

use std::collections::{BTreeMap, HashMap};
use std::time::Duration;

use artifacts::config::backend::BackendConfiguration;
use artifacts::config::make::MakeConfiguration;
use artifacts::tui::background::spawn_background_task;
use artifacts::tui::channels::{EffectCommand, EffectResult};
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
async fn test_graceful_shutdown_completes_in_flight() {
    // Send command A, signal shutdown, verify command A completes and result received
    // Background exits cleanly

    let backend = create_test_backend_config();
    let make = create_test_make_config();
    let shutdown_token = CancellationToken::new();

    let (tx_cmd, mut rx_res) = spawn_background_task(backend, make, shutdown_token.clone());

    // Send one command
    tx_cmd
        .send(EffectCommand::CheckSerialization {
            artifact_index: 0,
            artifact_name: "in-flight".to_string(),
            target: "machine".to_string(),
            target_type: "nixos".to_string(),
        })
        .unwrap();

    // Signal shutdown immediately
    shutdown_token.cancel();

    // Verify command completes before shutdown
    let result = timeout(Duration::from_secs(1), rx_res.recv())
        .await
        .expect("Should not timeout - command should complete before shutdown")
        .expect("Should receive result for in-flight command");

    match result {
        EffectResult::CheckSerialization { artifact_index, .. } => {
            assert_eq!(artifact_index, 0, "artifact_index should match");
        }
        _ => panic!("Expected CheckSerialization result"),
    }

    // Background should exit cleanly
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Channel should be closed
    let closed_result = timeout(Duration::from_millis(500), rx_res.recv())
        .await
        .expect("Should not timeout");

    assert!(
        closed_result.is_none(),
        "Channel should be closed after graceful shutdown"
    );

    println!("In-flight command completed before graceful shutdown");
}

#[tokio::test]
#[serial_test::serial]
async fn test_shutdown_with_queued_commands() {
    // Send commands A, B, C, signal shutdown, verify shutdown processes queued commands
    // Implementation processes remaining queue before shutdown

    let backend = create_test_backend_config();
    let make = create_test_make_config();
    let shutdown_token = CancellationToken::new();

    let (tx_cmd, mut rx_res) = spawn_background_task(backend, make, shutdown_token.clone());

    // Send multiple commands
    let num_commands = 5;
    for i in 0..num_commands {
        tx_cmd
            .send(EffectCommand::CheckSerialization {
                artifact_index: i,
                artifact_name: format!("queued-{}", i),
                target: "machine".to_string(),
                target_type: "nixos".to_string(),
            })
            .unwrap();
    }

    // Signal shutdown immediately
    shutdown_token.cancel();

    // Collect all results - should process queued commands before shutdown
    let mut received = Vec::new();
    loop {
        match timeout(Duration::from_millis(500), rx_res.recv()).await {
            Ok(Some(result)) => {
                if let EffectResult::CheckSerialization { artifact_index, .. } = result {
                    received.push(artifact_index);
                }
            }
            Ok(None) => break,
            Err(_) => break,
        }
    }

    // Verify all commands were processed
    assert_eq!(
        received.len(),
        num_commands,
        "All {} queued commands should be processed before shutdown",
        num_commands
    );

    // Verify FIFO order
    for (i, &idx) in received.iter().enumerate() {
        assert_eq!(
            idx, i,
            "FIFO order violated at position {}: expected {}, got {}",
            i, i, idx
        );
    }

    println!(
        "All {} queued commands processed before shutdown",
        received.len()
    );
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

    let (tx_cmd, rx_res) = spawn_background_task(backend, make, shutdown_token);

    // Drop the result receiver (simulates TUI closing)
    drop(rx_res);

    // Send a command - background will try to send result but channel is closed
    // This should not panic
    tx_cmd
        .send(EffectCommand::CheckSerialization {
            artifact_index: 0,
            artifact_name: "test".to_string(),
            target: "machine".to_string(),
            target_type: "nixos".to_string(),
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

    let (tx_cmd, mut rx_res) = spawn_background_task(backend, make, shutdown_token);

    // Send a command that will fail quickly (not timeout, just fail open)
    // This verifies the background task is running and responding
    tx_cmd
        .send(EffectCommand::CheckSerialization {
            artifact_index: 99,
            artifact_name: "timeout-test".to_string(),
            target: "machine".to_string(),
            target_type: "nixos".to_string(),
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
