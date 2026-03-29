//! Channel communication tests for TUI foreground/background communication.
//!
//! These tests verify that tokio mpsc unbounded channels work correctly
//! for async communication between foreground and background tasks.

use tokio::sync::mpsc::unbounded_channel;
use tokio::time::{Duration, timeout};

/// Mock command for testing channel communication
#[derive(Debug, Clone, PartialEq)]
pub enum MockCommand {
    CheckSerialization { artifact_index: usize, name: String },
    RunGenerator { artifact_index: usize, name: String },
}

/// Mock result for testing channel communication
#[derive(Debug, Clone, PartialEq)]
pub enum MockResult {
    CheckFinished {
        artifact_index: usize,
        needs_generation: bool,
    },
}

#[tokio::test]
async fn test_command_sent_via_channel() {
    // Verify commands can be sent from "foreground" to "background"
    // through unbounded channel
    let (tx_cmd, mut rx_cmd) = unbounded_channel::<MockCommand>();

    let cmd = MockCommand::CheckSerialization {
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
        MockCommand::CheckSerialization {
            artifact_index: 0,
            name: "test-artifact".to_string(),
        }
    );
}

#[tokio::test]
async fn test_result_received_via_channel() {
    // Verify results can be received from "background" back to "foreground"
    let (tx_res, mut rx_res) = unbounded_channel::<MockResult>();

    let result = MockResult::CheckFinished {
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
        MockResult::CheckFinished {
            artifact_index: 1,
            needs_generation: true,
        }
    );
}

#[tokio::test]
async fn test_channel_disconnect_graceful() {
    // Verify dropping sender causes recv() to return None, handled gracefully
    let (tx_cmd, mut rx_cmd) = unbounded_channel::<MockCommand>();

    tx_cmd
        .send(MockCommand::CheckSerialization {
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
        MockCommand::CheckSerialization {
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
    let (tx_cmd, mut rx_cmd) = unbounded_channel::<MockCommand>();

    // Send commands A, B, C with sequential indices
    for i in 0..3 {
        tx_cmd
            .send(MockCommand::RunGenerator {
                artifact_index: i,
                name: format!("artifact-{}", i),
            })
            .unwrap();
    }

    // Drop sender (done sending)
    drop(tx_cmd);

    // Receive and verify FIFO order
    for i in 0..3 {
        let received = match timeout(Duration::from_millis(100), rx_cmd.recv()).await {
            Ok(Some(cmd)) => cmd,
            Ok(None) => panic!("Should receive message {}", i),
            Err(e) => panic!("Should receive message {}: {:?}", i, e),
        };

        assert_eq!(
            received,
            MockCommand::RunGenerator {
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
