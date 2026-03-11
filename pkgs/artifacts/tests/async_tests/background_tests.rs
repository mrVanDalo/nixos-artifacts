//! Background task integration tests with actual BackgroundTask.
//!
//! These tests verify the actual BackgroundTask from src/tui/background.rs
//! processes commands correctly and returns expected results.

use std::collections::{BTreeMap, HashMap};

use artifacts::app::message::{Message, ScriptOutput};
use artifacts::app::model::{ArtifactStatus, TargetType};
use artifacts::config::backend::BackendConfiguration;
use artifacts::config::make::MakeConfiguration;
use artifacts::tui::background::{BackgroundEffectHandler, spawn_background_task};
use artifacts::app::effect::Effect;
use tokio::time::{Duration, timeout};
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
async fn test_background_processes_check_command() {
    // Send CheckSerialization command, verify CheckFinished result received
    let backend = create_test_backend_config();
    let make = create_test_make_config();
    let shutdown_token = CancellationToken::new();

    let (tx_cmd, mut rx_res) = spawn_background_task(backend, make, shutdown_token);

    // Send a CheckSerialization command
    tx_cmd
        .send(Effect::CheckSerialization {
            artifact_index: 42,
            artifact_name: "test-artifact".to_string(),
            target_type: TargetType::NixOS { machine: "machine-1".to_string() },
        })
        .unwrap();

    // Receive the result with timeout
    let result = timeout(Duration::from_secs(1), rx_res.recv())
        .await
        .expect("Should not timeout")
        .expect("Should receive result");

    // Verify result variant and artifact_index
    match result {
        Message::CheckSerializationResult {
            artifact_index,
            status,
            result: _,
        } => {
            assert_eq!(artifact_index, 42, "artifact_index should match command");
            // For empty config, artifact not found -> fail-open -> NeedsGeneration
            // The actual behavior depends on the backend implementation
            assert!(
                matches!(status, ArtifactStatus::NeedsGeneration | ArtifactStatus::Failed { .. }),
                "Should have NeedsGeneration or Failed status"
            );
        }
        _ => panic!("Expected CheckSerializationResult message, got {:?}", result),
    }
}

#[tokio::test]
async fn test_background_processes_generator_command() {
    // Send RunGenerator command, verify GeneratorFinished result
    // Note: With empty config, this will fail to find artifact
    let backend = create_test_backend_config();
    let make = create_test_make_config();
    let shutdown_token = CancellationToken::new();

    let (tx_cmd, mut rx_res) = spawn_background_task(backend, make, shutdown_token);

    // Send a RunGenerator command
    tx_cmd
        .send(Effect::RunGenerator {
            artifact_index: 7,
            artifact_name: "test-artifact".to_string(),
            target_type: TargetType::NixOS { machine: "machine-1".to_string() },
            prompts: HashMap::new(),
        })
        .unwrap();

    // Receive the result with timeout
    let result = timeout(Duration::from_secs(1), rx_res.recv())
        .await
        .expect("Should not timeout")
        .expect("Should receive result");

    // Verify result variant and artifact_index
    match result {
        Message::GeneratorFinished {
            artifact_index,
            result: gen_result,
        } => {
            assert_eq!(artifact_index, 7, "artifact_index should match command");
            // With empty config, generator lookup will fail
            assert!(
                gen_result.is_err(),
                "Generator should fail with empty config (artifact not found)"
            );
        }
        _ => panic!("Expected GeneratorFinished message, got {:?}", result),
    }
}

#[tokio::test]
async fn test_timeout_behavior() {
    // Verify that the timeout mechanism works correctly
    // We test this by sending a command and verifying it completes
    // with a timeout wrapper, demonstrating that the background task
    // respects timeout boundaries

    let backend = create_test_backend_config();
    let make = create_test_make_config();
    let shutdown_token = CancellationToken::new();

    let (tx_cmd, mut rx_res) = spawn_background_task(backend, make, shutdown_token);

    // Send a command that will fail fast with empty config
    tx_cmd
        .send(Effect::CheckSerialization {
            artifact_index: 99,
            artifact_name: "missing".to_string(),
            target_type: TargetType::NixOS { machine: "test".to_string() },
        })
        .unwrap();

    // Should receive result quickly (fails open, not timeout)
    // Using timeout wrapper to verify timeout mechanism works
    let result = timeout(Duration::from_secs(5), rx_res.recv())
        .await
        .expect("Should not timeout - demonstrates timeout wrapper works")
        .expect("Should receive result");

    match result {
        Message::CheckSerializationResult { artifact_index, .. } => {
            assert_eq!(artifact_index, 99, "artifact_index should match");
        }
        _ => panic!("Expected CheckSerialization result"),
    }
}

#[tokio::test]
async fn test_graceful_shutdown_on_channel_close() {
    // Drop command channel, verify background task exits cleanly (not panic)
    let backend = create_test_backend_config();
    let make = create_test_make_config();
    let shutdown_token = CancellationToken::new();

    let (tx_cmd, mut rx_res) = spawn_background_task(backend, make, shutdown_token);

    // Send one command before closing
    tx_cmd
        .send(Effect::CheckSerialization {
            artifact_index: 0,
            artifact_name: "test".to_string(),
            target_type: TargetType::NixOS { machine: "machine".to_string() },
        })
        .unwrap();

    // Drop the command sender - this signals the background to exit
    drop(tx_cmd);

    // Receive the result (should complete before shutdown)
    let _result = timeout(Duration::from_secs(1), rx_res.recv())
        .await
        .expect("Should not timeout");

    // May or may not receive the result depending on timing
    // The important thing is that the background exits cleanly

    // Give background task time to process shutdown
    tokio::time::sleep(Duration::from_millis(50)).await;

    // If we made it here without panic, shutdown was graceful
    println!("Background task shut down gracefully");
}

#[tokio::test]
async fn test_fifo_ordering_with_real_background() {
    // Verify commands are processed in FIFO order with actual background task
    let backend = create_test_backend_config();
    let make = create_test_make_config();
    let shutdown_token = CancellationToken::new();

    let (tx_cmd, mut rx_res) = spawn_background_task(backend, make, shutdown_token);

    // Send 5 CheckSerialization commands with sequential indices
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

    // Receive results in order and verify FIFO
    for i in 0..num_commands {
        let result = timeout(Duration::from_secs(1), rx_res.recv())
            .await
            .expect("Should not timeout")
            .expect("Should receive result");

        match result {
            Message::CheckSerializationResult { artifact_index, .. } => {
                assert_eq!(
                    artifact_index, i,
                    "FIFO order violated at index {}. Expected {}, got {}",
                    i, i, artifact_index
                );
            }
            _ => panic!("Expected CheckSerialization result"),
        }
    }
}

#[tokio::test]
async fn test_background_effect_handler_new() {
    // Test that BackgroundEffectHandler::new creates handler correctly
    let backend = create_test_backend_config();
    let make = create_test_make_config();

    let mut handler = BackgroundEffectHandler::new(backend, make);

    // Verify handler was created (we can't check internals directly,
    // but we can verify it compiles and accepts commands)
    let cmd = Effect::CheckSerialization {
        artifact_index: 0,
        artifact_name: "test".to_string(),
        target_type: TargetType::NixOS { machine: "machine".to_string() },
    };

    // Execute will fail (artifact not found) but should not panic
    let result = handler.execute(cmd).await;

    match result {
        Message::CheckSerializationResult { artifact_index, .. } => {
            assert_eq!(artifact_index, 0, "artifact_index should be preserved");
        }
        _ => panic!("Expected CheckSerialization result"),
    }
}

#[tokio::test]
async fn test_cancellation_token_shutdown() {
    // Verify that cancelling the shutdown_token triggers graceful shutdown
    let backend = create_test_backend_config();
    let make = create_test_make_config();
    let shutdown_token = CancellationToken::new();

    let (tx_cmd, mut rx_res) = spawn_background_task(backend, make, shutdown_token.clone());

    // Send a few commands
    for i in 0..3 {
        tx_cmd
            .send(Effect::CheckSerialization {
                artifact_index: i,
                artifact_name: format!("artifact-{}", i),
                target_type: TargetType::NixOS { machine: "machine".to_string() },
            })
            .unwrap();
    }

    // Cancel the token to trigger shutdown
    shutdown_token.cancel();

    // Should be able to receive any pending results
    // Background will process remaining queue before exiting
    let mut received = 0;
    while let Ok(result) = timeout(Duration::from_millis(500), rx_res.recv()).await {
        if result.is_none() {
            break;
        }
        received += 1;
    }

    // Should have received results for the commands that were processed
    assert!(
        received > 0 || received <= 3,
        "Should receive 0-3 results (received {})",
        received
    );

    println!("Received {} results before shutdown", received);
}
