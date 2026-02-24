//! Channel-based async communication for TUI foreground/background tasks.
//!
//! This module provides the message types that enable async communication between
//! the TUI foreground (main thread) and background task (effect executor).
//!
//! # Architecture
//!
//! The channel system follows a command/result pattern:
//! - **Foreground → Background:** `EffectCommand` messages tell the background what to execute
//! - **Background → Foreground:** `EffectResult` messages return execution outcomes
//!
//! ## Design Decisions
//!
//! - **Unbounded channels:** No backpressure - TUI never blocks on send
//!   (See 01-CONTEXT.md: "Channel capacity: Unbounded")
//! - **artifact_index in every message:** Enables dispatch back to correct model entry
//!   (See 01-CONTEXT.md: "Message content: Include artifact ID")
//! - **Buffered output:** Complete output returned at end, not streamed
//!   (See 01-CONTEXT.md: "Script output: Complete output returned at end")
//! - **Errors in results:** Errors travel in result messages, not separate channel
//!   (See 01-CONTEXT.md: "Error handling: Errors travel in result messages")
//!
//! ## Usage
//!
//! ```rust
//! use artifacts::tui::background::spawn_background_task;
//!
//! // Spawn background task
//! let (tx_cmd, rx_cmd) = tokio::sync::mpsc::unbounded_channel();
//! let (tx_res, rx_res) = tokio::sync::mpsc::unbounded_channel();
//! spawn_background_task(backend, make, shutdown); // from background.rs
//!
//! // Send command from foreground
//! tx_cmd.send(EffectCommand::CheckSerialization { ... });
//!
//! // Receive result in foreground
//! let result = rx_res.recv().await;
//! ```

use crate::backend::output_capture::{CapturedOutput, OutputStream as BackendOutputStream};
use std::collections::HashMap;

/// Structured script output preserving stdout/stderr separation
#[derive(Debug, Clone, Default)]
pub struct ScriptOutput {
    pub stdout_lines: Vec<String>,
    pub stderr_lines: Vec<String>,
}

impl ScriptOutput {
    /// Convert from CapturedOutput to ScriptOutput, splitting stdout and stderr
    pub fn from_captured(captured: &CapturedOutput) -> Self {
        let mut stdout_lines = Vec::new();
        let mut stderr_lines = Vec::new();

        for line in &captured.lines {
            match line.stream {
                BackendOutputStream::Stdout => stdout_lines.push(line.content.clone()),
                BackendOutputStream::Stderr => stderr_lines.push(line.content.clone()),
            }
        }

        Self {
            stdout_lines,
            stderr_lines,
        }
    }

    /// Create a ScriptOutput from a single message (for errors/warnings)
    pub fn from_message(message: &str) -> Self {
        Self {
            stdout_lines: vec![message.to_string()],
            stderr_lines: Vec::new(),
        }
    }
}

/// Commands sent from foreground to background task.
///
/// Each variant includes `artifact_index` to enable dispatch of results
/// back to the correct model entry. The `artifact_name` is included for
/// logging and display purposes.
#[derive(Debug, Clone)]
pub enum EffectCommand {
    /// Check if an artifact needs regeneration
    CheckSerialization {
        artifact_index: usize,
        artifact_name: String,
        target: String,
        target_type: String, // "nixos" or "home"
    },

    /// Run the generator script for an artifact
    RunGenerator {
        artifact_index: usize,
        artifact_name: String,
        target: String,
        target_type: String,
        prompts: HashMap<String, String>,
    },

    /// Serialize the generated files
    Serialize {
        artifact_index: usize,
        artifact_name: String,
        target: String,
        target_type: String,
    },

    /// Check if a shared artifact needs regeneration
    SharedCheckSerialization {
        artifact_index: usize,
        artifact_name: String,
        targets: Vec<String>,
        target_types: Vec<String>,
    },

    /// Run the generator script for a shared artifact
    RunSharedGenerator {
        artifact_index: usize,
        artifact_name: String,
        machine_targets: Vec<String>,
        user_targets: Vec<String>,
        prompts: HashMap<String, String>,
    },

    /// Serialize the generated files for a shared artifact
    SharedSerialize {
        artifact_index: usize,
        artifact_name: String,
        machine_targets: Vec<String>,
        user_targets: Vec<String>,
    },
}

/// Identifies which stream a line came from
#[derive(Debug, Clone, Copy)]
pub enum OutputStream {
    Stdout,
    Stderr,
}

/// Results sent from background task back to foreground.
///
/// Each variant corresponds to an `EffectCommand` and includes `artifact_index`
/// for dispatch. Errors are encoded as `bool+Option<String>` rather than Result
/// to avoid serialization issues over channels.
#[derive(Debug, Clone)]
pub enum EffectResult {
    /// Result of check_serialization command
    CheckSerialization {
        artifact_index: usize,
        needs_generation: bool,
        exists: bool,
        output: ScriptOutput,
    },

    /// Result of generator script execution
    GeneratorFinished {
        artifact_index: usize,
        success: bool,
        output: ScriptOutput,
        error: Option<String>,
    },

    /// Result of serialize script execution
    SerializeFinished {
        artifact_index: usize,
        success: bool,
        output: ScriptOutput,
        error: Option<String>,
    },

    /// Result of shared check_serialization command
    SharedCheckSerialization {
        artifact_index: usize,
        needs_generation: Vec<bool>, // One per target
        exists: Vec<bool>,           // One per target
        outputs: Vec<ScriptOutput>,
    },

    /// Result of shared generator script execution
    SharedGeneratorFinished {
        artifact_index: usize,
        success: bool,
        output: ScriptOutput,
        error: Option<String>,
    },

    /// Result of shared serialize script execution
    SharedSerializeFinished {
        artifact_index: usize,
        results: Vec<(String, bool, ScriptOutput)>, // (target, success, output)
    },

    /// Streaming output line received during script execution
    OutputLine {
        artifact_index: usize,
        stream: OutputStream,
        content: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_effect_command_has_artifact_index() {
        // Verify CheckSerialization variant has artifact_index
        let cmd = EffectCommand::CheckSerialization {
            artifact_index: 5,
            artifact_name: "test".to_string(),
            target: "machine".to_string(),
            target_type: "nixos".to_string(),
        };
        match cmd {
            EffectCommand::CheckSerialization { artifact_index, .. } => {
                assert_eq!(artifact_index, 5);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_effect_result_has_artifact_index() {
        // Verify CheckSerialization result has artifact_index
        let res = EffectResult::CheckSerialization {
            artifact_index: 3,
            needs_generation: true,
            exists: false,
            output: ScriptOutput::default(),
        };
        match res {
            EffectResult::CheckSerialization { artifact_index, .. } => {
                assert_eq!(artifact_index, 3);
            }
            _ => panic!("Wrong variant"),
        }
    }

    #[test]
    fn test_all_effect_command_variants_have_artifact_index() {
        // Test that we can create and match on all variants with artifact_index
        let commands = [
            EffectCommand::CheckSerialization {
                artifact_index: 0,
                artifact_name: "test".to_string(),
                target: "machine".to_string(),
                target_type: "nixos".to_string(),
            },
            EffectCommand::RunGenerator {
                artifact_index: 1,
                artifact_name: "test".to_string(),
                target: "machine".to_string(),
                target_type: "nixos".to_string(),
                prompts: HashMap::new(),
            },
            EffectCommand::Serialize {
                artifact_index: 2,
                artifact_name: "test".to_string(),
                target: "machine".to_string(),
                target_type: "nixos".to_string(),
            },
            EffectCommand::SharedCheckSerialization {
                artifact_index: 3,
                artifact_name: "test".to_string(),
                targets: vec!["machine1".to_string()],
                target_types: vec!["nixos".to_string()],
            },
            EffectCommand::RunSharedGenerator {
                artifact_index: 4,
                artifact_name: "test".to_string(),
                machine_targets: vec!["machine1".to_string()],
                user_targets: vec![],
                prompts: HashMap::new(),
            },
            EffectCommand::SharedSerialize {
                artifact_index: 5,
                artifact_name: "test".to_string(),
                machine_targets: vec!["machine1".to_string()],
                user_targets: vec![],
            },
        ];

        for (i, cmd) in commands.iter().enumerate() {
            let idx = match cmd {
                EffectCommand::CheckSerialization { artifact_index, .. } => *artifact_index,
                EffectCommand::RunGenerator { artifact_index, .. } => *artifact_index,
                EffectCommand::Serialize { artifact_index, .. } => *artifact_index,
                EffectCommand::SharedCheckSerialization { artifact_index, .. } => *artifact_index,
                EffectCommand::RunSharedGenerator { artifact_index, .. } => *artifact_index,
                EffectCommand::SharedSerialize { artifact_index, .. } => *artifact_index,
            };
            assert_eq!(idx, i, "artifact_index should match position");
        }
    }

    #[test]
    fn test_all_effect_result_variants_have_artifact_index() {
        // Test that we can create and match on all result variants with artifact_index
        let results = [
            EffectResult::CheckSerialization {
                artifact_index: 0,
                needs_generation: true,
                exists: false,
                output: ScriptOutput::default(),
            },
            EffectResult::GeneratorFinished {
                artifact_index: 1,
                success: true,
                output: ScriptOutput::default(),
                error: None,
            },
            EffectResult::SerializeFinished {
                artifact_index: 2,
                success: true,
                output: ScriptOutput::default(),
                error: None,
            },
            EffectResult::SharedCheckSerialization {
                artifact_index: 3,
                needs_generation: vec![true],
                exists: vec![false],
                outputs: vec![ScriptOutput::default()],
            },
            EffectResult::SharedGeneratorFinished {
                artifact_index: 4,
                success: true,
                output: ScriptOutput::default(),
                error: None,
            },
            EffectResult::SharedSerializeFinished {
                artifact_index: 5,
                results: vec![("target".to_string(), true, ScriptOutput::default())],
            },
        ];

        for (i, res) in results.iter().enumerate() {
            let idx = match res {
                EffectResult::CheckSerialization { artifact_index, .. } => *artifact_index,
                EffectResult::GeneratorFinished { artifact_index, .. } => *artifact_index,
                EffectResult::SerializeFinished { artifact_index, .. } => *artifact_index,
                EffectResult::SharedCheckSerialization { artifact_index, .. } => *artifact_index,
                EffectResult::SharedGeneratorFinished { artifact_index, .. } => *artifact_index,
                EffectResult::SharedSerializeFinished { artifact_index, .. } => *artifact_index,
                EffectResult::OutputLine { artifact_index, .. } => *artifact_index,
            };
            assert_eq!(idx, i, "artifact_index should match position");
        }
    }
}
