//! Channel communication tests for TUI foreground/background communication.
//!
//! These tests verify that tokio mpsc unbounded channels work correctly
//! for async communication between foreground and background tasks.

use tokio::sync::mpsc::unbounded_channel;
use tokio::time::{timeout, Duration};

/// Mock EffectCommand for testing channel communication
#[derive(Debug, Clone, PartialEq)]
pub enum MockEffectCommand {
    CheckSerialization { artifact_index: usize, name: String },
    RunGenerator { artifact_index: usize, name: String },
}

/// Mock EffectResult for testing channel communication
#[derive(Debug, Clone, PartialEq)]
pub enum MockEffectResult {
    CheckFinished { artifact_index: usize, needs_generation: bool },
    GeneratorFinished { artifact_index: usize, success: bool },
}

#[tokio::test]
async fn test_command_sent_via_channel() {
    // Verify EffectCommand can be sent from "foreground" to "background"
    // through unbounded channel
    let (tx_cmd, mut rx_cmd) = unbounded_channel::<MockEffectCommand>();

    let cmd = MockEffectCommand::CheckSerialization {
        artifact_index: 0,
        name: "test-artifact".to_string(),
    };

    tx_cmd.send(cmd).unwrap();

    // Receive with timeout to prevent hanging
    let received = timeout(Duration::from_millis(100), rx_cmd.recv())
        .await
        .expect("Should not timeout")
        .expect("Should receive message");

    assert_eq!(
        received,
        MockEffectCommand::CheckSerialization {
            artifact_index: 0,
            name: "test-artifact".to_string(),
        }
    );
}

#[tokio::test]
async fn test_result_received_via_channel() {
    // Verify EffectResult can be received from "background" back to "foreground"
    let (tx_res, mut rx_res) = unbounded_channel::<MockEffectResult>();

    let result = MockEffectResult::CheckFinished {
        artifact_index: 1,
        needs_generation: true,
    };

    tx_res.send(result).unwrap();

    // Receive with timeout to prevent hanging
    let received = timeout(Duration::from_millis(100), rx_res.recv())
        .await
        .expect("Should not timeout")
        .expect("Should receive message");

    assert_eq!(
        received,
        MockEffectResult::CheckFinished {
            artifact_index: 1,
            needs_generation: true,
        }
    );
}

#[tokio::test]
async fn test_channel_disconnect_graceful() {
    // Verify dropping sender causes recv() to return None, handled gracefully
    let (tx_cmd, mut rx_cmd) = unbounded_channel::<MockEffectCommand>();

    tx_cmd.send(MockEffectCommand::CheckSerialization {
        artifact_index: 0,
        name: "first".to_string(),
    })
    .unwrap();

    // Drop the sender (simulates TUI closing or background exiting)
    drop(tx_cmd);

    // Should be able to receive the pending message
    let received = timeout(Duration::from_millis(100), rx_cmd.recv())
        .await
        .expect("Should not timeout")
        .expect("Should receive pending message");

    assert_eq!(
        received,
        MockEffectCommand::CheckSerialization {
            artifact_index: 0,
            name: "first".to_string(),
        }
    );

    // Next recv should return None (channel closed)
    let closed_result = timeout(Duration::from_millis(100), rx_cmd.recv())
        .await
        .expect("Should not timeout");

    assert!(
        closed_result.is_none(),
        "Channel should be closed and return None"
    );
}

#[tokio::test]
async fn test_multiple_commands_sequential() {
    // Verify FIFO ordering: commands sent A, B, C are received in same order
    let (tx_cmd, mut rx_cmd) = unbounded_channel::<MockEffectCommand>();

    // Send commands A, B, C with sequential indices
    for i in 0..3 {
        tx_cmd
            .send(MockEffectCommand::RunGenerator {
                artifact_index: i,
                name: format!("artifact-{}", i),
            })
            .unwrap();
    }

    // Drop sender (done sending)
    drop(tx_cmd);

    // Receive and verify FIFO order
    for i in 0..3 {
        let received = timeout(Duration::from_millis(100), rx_cmd.recv())
            .await
            .expect("Should not timeout")
            .expect(&format!("Should receive message {}", i));

        assert_eq!(
            received,
            MockEffectCommand::RunGenerator {
                artifact_index: i,
                name: format!("artifact-{}", i),
            },
            "FIFO order violated at index {}",
            i
        );
    }

    // Channel should be closed
    let closed_result = timeout(Duration::from_millis(100), rx_cmd.recv())
        .await
        .expect("Should not timeout");
    assert!(closed_result.is_none(), "Channel should be closed");
}
