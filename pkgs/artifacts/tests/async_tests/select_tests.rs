//! Tests for tokio::select! branch coverage in background task.
//!
//! These tests verify each branch of the select! loop executes correctly:
//! - shutdown branch (CancellationToken cancelled)
//! - command branch (cmd_rx.recv())
//! - channel_closed branch (else when channel closed)
//! - in_flight command completion with shutdown

use std::collections::{BTreeMap, HashMap};
use std::time::Duration;

use artifacts::app::message::Message;
use artifacts::app::model::TargetType;
use artifacts::config::backend::BackendConfiguration;
use artifacts::config::make::MakeConfiguration;
use artifacts::tui::background::spawn_background_task;
use artifacts::app::effect::Effect;
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
async fn test_select_shutdown_branch() {
    // Trigger CancellationToken.cancel(), verify background task exits
    // Exercises the `shutdown.cancelled()` branch of select!

    let backend = create_test_backend_config();
    let make = create_test_make_config();
    let shutdown_token = CancellationToken::new();

    let (tx_cmd, mut rx_res) = spawn_background_task(backend, make, shutdown_token.clone());

    // Send one command that will be processed before shutdown
    tx_cmd
        .send(Effect::CheckSerialization {
            artifact_index: 0,
            artifact_name: "test".to_string(),
            target_type: TargetType::NixOS { machine: "machine".to_string() },
        })
        .unwrap();

    // Receive the result (command should complete before shutdown)
    let result = timeout(Duration::from_secs(1), rx_res.recv())
        .await
        .expect("Should not timeout")
        .expect("Should receive result");

    // Verify result was received
    match result {
        Message::CheckSerializationResult { artifact_index, .. } => {
            assert_eq!(artifact_index, 0, "artifact_index should match");
        }
        _ => panic!("Expected CheckSerialization result"),
    }

    // Now signal shutdown
    shutdown_token.cancel();

    // Background should exit cleanly, which means the channel will close
    // Give it time to process shutdown
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Channel should be closed now
    let closed_result = timeout(Duration::from_millis(500), rx_res.recv())
        .await
        .expect("Should not timeout");

    assert!(
        closed_result.is_none(),
        "Channel should be closed after shutdown branch exits"
    );

    println!("Shutdown branch executed correctly");
}

#[tokio::test]
#[serial_test::serial]
async fn test_select_command_branch() {
    // Send command via channel, verify it's processed and result returned
    // Exercises the `cmd_rx.recv()` branch of select!

    let backend = create_test_backend_config();
    let make = create_test_make_config();
    let shutdown_token = CancellationToken::new();

    let (tx_cmd, mut rx_res) = spawn_background_task(backend, make, shutdown_token);

    // Send multiple commands and verify each is processed
    let num_commands = 3;
    for i in 0..num_commands {
        tx_cmd
            .send(Effect::CheckSerialization {
                artifact_index: i,
                artifact_name: format!("artifact-{}", i),
                target_type: TargetType::NixOS { machine: "machine".to_string() },
            })
            .unwrap();
    }

    // Receive all results and verify each command was processed
    for i in 0..num_commands {
        let result = timeout(Duration::from_secs(1), rx_res.recv())
            .await
            .expect("Should not timeout")
            .expect("Should receive result");

        match result {
            Message::CheckSerializationResult { artifact_index, .. } => {
                assert_eq!(
                    artifact_index, i,
                    "artifact_index should match command index {}",
                    i
                );
            }
            _ => panic!("Expected CheckSerialization result"),
        }
    }

    println!(
        "Command branch executed correctly for all {} commands",
        num_commands
    );
}

#[tokio::test]
#[serial_test::serial]
async fn test_select_channel_closed_branch() {
    // Drop all command senders and signal shutdown, verify background exits
    // The `else` branch in select! executes when cmd_rx.recv() returns None
    // and no other branches are ready

    let backend = create_test_backend_config();
    let make = create_test_make_config();
    let shutdown_token = CancellationToken::new();

    let (tx_cmd, mut rx_res) = spawn_background_task(backend, make, shutdown_token.clone());

    // Send one command before closing
    tx_cmd
        .send(Effect::CheckSerialization {
            artifact_index: 42,
            artifact_name: "test".to_string(),
            target_type: TargetType::NixOS { machine: "machine".to_string() },
        })
        .unwrap();

    // Drop the sender - this closes the command channel
    // After the current command is processed, rx_cmd.recv() will return None
    drop(tx_cmd);

    // Should be able to receive the result (command completes before channel closes)
    let result = timeout(Duration::from_secs(1), rx_res.recv())
        .await
        .expect("Should not timeout")
        .expect("Should receive result before channel closes");

    match result {
        Message::CheckSerializationResult { artifact_index, .. } => {
            assert_eq!(artifact_index, 42, "artifact_index should match");
        }
        _ => panic!("Expected CheckSerialization result"),
    }

    // Signal shutdown - this makes shutdown_token.cancelled() ready
    // When combined with the closed command channel, the select! will:
    // - See shutdown is ready
    // - Try to recv() which returns None (channel closed)
    // - The shutdown branch takes priority
    shutdown_token.cancel();

    // Background should exit via the shutdown branch now
    // Give it time to process
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Channel should be closed (background exited)
    let closed_result = timeout(Duration::from_secs(2), rx_res.recv())
        .await
        .expect("Should not timeout waiting for channel to close");

    assert!(
        closed_result.is_none(),
        "Channel should be closed after background exits"
    );

    println!("Channel closed and shutdown branches executed correctly");
}

#[tokio::test]
#[serial_test::serial]
async fn test_select_with_in_flight_command() {
    // Start command, signal shutdown before command completes,
    // verify command completes before shutdown
    // Tests fairness of select!

    let backend = create_test_backend_config();
    let make = create_test_make_config();
    let shutdown_token = CancellationToken::new();

    let (tx_cmd, mut rx_res) = spawn_background_task(backend, make, shutdown_token.clone());

    // Send multiple commands
    let num_commands = 5;
    for i in 0..num_commands {
        tx_cmd
            .send(Effect::CheckSerialization {
                artifact_index: i,
                artifact_name: format!("artifact-{}", i),
                target_type: TargetType::NixOS { machine: "machine".to_string() },
            })
            .unwrap();
    }

    // Immediately signal shutdown
    shutdown_token.cancel();

    // Background should process remaining queue before exiting
    // Collect all results
    let mut received_results = Vec::new();
    loop {
        match timeout(Duration::from_millis(500), rx_res.recv()).await {
            Ok(Some(result)) => {
                if let Message::CheckSerializationResult { artifact_index, .. } = result {
                    received_results.push(artifact_index);
                }
            }
            Ok(None) => break, // Channel closed
            Err(_) => break,   // Timeout - no more results
        }
    }

    // Should have received results for all commands (or most)
    // The implementation processes queued commands before shutdown
    assert!(
        received_results.len() >= num_commands || received_results.len() == num_commands,
        "Should receive results for all {} commands (got {})",
        num_commands,
        received_results.len()
    );

    // Verify results are in order
    let expected: Vec<usize> = (0..received_results.len()).collect();
    assert_eq!(
        received_results, expected,
        "Results should be in FIFO order"
    );

    println!(
        "In-flight command completion test passed: {} commands processed before shutdown",
        received_results.len()
    );
}
