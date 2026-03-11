//! Channel-based async communication for TUI foreground/background tasks.
//!
//! This module provides the channel setup and utility types for async communication
//! between the TUI foreground (main thread) and background task (effect executor).
//!
//! # Architecture
//!
//! The channel system follows a command/result pattern:
//! - **Foreground → Background:** `Effect` messages tell the background what to execute
//! - **Background → Foreground:** `Message` messages return execution outcomes
//!
//! ## Design Decisions
//!
//! - **Unbounded channels:** No backpressure - TUI never blocks on send
//! - **artifact_index in every message:** Enables dispatch back to correct model entry
//! - **Buffered output:** Complete output returned at end, not streamed
//! - **Errors in results:** Errors travel in result messages, not separate channel

/// Re-export Effect for channel communication
pub use crate::app::effect::Effect;

/// Re-export Message for channel communication  
pub use crate::app::message::Message;

/// Re-export ScriptOutput for convenience
pub use crate::app::message::ScriptOutput;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::model::TargetType;

    #[test]
    fn test_effect_has_artifact_index() {
        let cmd = Effect::CheckSerialization {
            artifact_index: 5,
            artifact_name: "test".to_string(),
            target_type: TargetType::NixOS { machine: "machine".to_string() },
        };
        match cmd {
            Effect::CheckSerialization { artifact_index, .. } => {
                assert_eq!(artifact_index, 5);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_all_effect_variants_have_artifact_index() {
        let commands = [
            Effect::CheckSerialization {
                artifact_index: 0,
                artifact_name: "test".to_string(),
                target_type: TargetType::NixOS { machine: "machine".to_string() },
            },
            Effect::RunGenerator {
                artifact_index: 1,
                artifact_name: "test".to_string(),
                target_type: TargetType::NixOS { machine: "machine".to_string() },
                prompts: std::collections::HashMap::new(),
            },
            Effect::Serialize {
                artifact_index: 2,
                artifact_name: "test".to_string(),
                target_type: TargetType::NixOS { machine: "machine".to_string() },
            },
            Effect::SharedCheckSerialization {
                artifact_index: 3,
                artifact_name: "test".to_string(),
                nixos_targets: vec!["machine1".to_string()],
                home_targets: vec![],
            },
            Effect::RunSharedGenerator {
                artifact_index: 4,
                artifact_name: "test".to_string(),
                prompts: std::collections::HashMap::new(),
            },
            Effect::SharedSerialize {
                artifact_index: 5,
                artifact_name: "test".to_string(),
                nixos_targets: vec!["machine1".to_string()],
                home_targets: vec![],
            },
        ];

        for (i, cmd) in commands.iter().enumerate() {
            let idx = match cmd {
                Effect::CheckSerialization { artifact_index, .. } => *artifact_index,
                Effect::RunGenerator { artifact_index, .. } => *artifact_index,
                Effect::Serialize { artifact_index, .. } => *artifact_index,
                Effect::SharedCheckSerialization { artifact_index, .. } => *artifact_index,
                Effect::RunSharedGenerator { artifact_index, .. } => *artifact_index,
                Effect::SharedSerialize { artifact_index, .. } => *artifact_index,
                Effect::None | Effect::Batch(_) | Effect::Quit => panic!("Unexpected variant"),
            };
            assert_eq!(idx, i, "artifact_index should match position");
        }
    }
}