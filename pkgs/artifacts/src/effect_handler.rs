//! EffectHandler bridge for TUI foreground/background communication.
//!
//! This module provides the EffectHandler struct that routes EffectCommands to the
//! background task and manages temporary directory state between effects.
//!
//! ## Architecture
//!
//! - **EffectHandler** lives in the TUI foreground (runtime thread)
//! - **BackgroundEffectHandler** lives in the background task (background.rs)
//! - Communication via tokio mpsc channels
//!
//! ## Temp Directory Lifecycle
//!
//! The temp directory flows through effects in this pattern:
//! 1. RunGenerator creates temp_dir, stores generator output
//! 2. GeneratorFinished result includes temp_dir path
//! 3. EffectHandler.store_temp_dir() preserves the TempDir object
//! 4. Serialize effect retrieves temp_dir via take_temp_dir()
//! 5. TempDir auto-drops after Serialize consumes it
//!
//! This ensures the temp directory survives from generator completion to
//! serialization while being properly cleaned up afterwards.

use std::collections::HashMap;

use tokio::sync::mpsc::UnboundedSender;

use crate::app::effect::Effect;
use crate::app::message::Msg;
use crate::app::model::OutputStream;
use crate::tui::channels::{EffectCommand, EffectResult, ScriptOutput};

/// Handler that routes effects to the background task.
///
/// This struct lives in the TUI foreground and manages:
/// - Sending EffectCommands to the background task via channels
/// - Receiving EffectResults and converting them to Msgs
/// - Storing temporary directories between effect boundaries
///
/// The temp directory management is critical: TempDir objects must be kept alive
/// until serialization completes, otherwise the directory gets deleted.
pub struct EffectHandler {
    /// Channel sender for dispatching commands to background task
    command_tx: UnboundedSender<EffectCommand>,
    /// Temporary directory storage between RunGenerator -> Serialize
    /// The TempDir is stored here after generator completes, then taken
    /// by the Serialize effect when it runs.
    current_temp_dir: Option<tempfile::TempDir>,
}

impl EffectHandler {
    /// Create a new EffectHandler with the given command channel.
    ///
    /// # Arguments
    ///
    /// * `command_tx` - Sender for EffectCommand messages to the background task
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let (tx_cmd, rx_res) = spawn_background_task(backend, make);
    /// let handler = EffectHandler::new(tx_cmd);
    /// ```
    pub fn new(command_tx: UnboundedSender<EffectCommand>) -> Self {
        Self {
            command_tx,
            current_temp_dir: None,
        }
    }

    /// Send an effect command to the background task.
    ///
    /// This method converts the Effect into an EffectCommand and sends it
    /// to the background task for execution. Returns an error if the channel
    /// is closed (background task exited).
    #[cfg(feature = "logging")]
    pub async fn run_effect(&mut self, effect: Effect) -> anyhow::Result<()> {
        crate::debug!("Sending effect to background: {:?}", effect);
        if let Some(cmd) = self.effect_to_command(effect) {
            self.command_tx
                .send(cmd)
                .map_err(|_| anyhow::anyhow!("Background task channel closed"))?;
        }
        Ok(())
    }

    /// Send an effect command to the background task (without logging feature).
    #[cfg(not(feature = "logging"))]
    pub async fn run_effect(&mut self, effect: Effect) -> anyhow::Result<()> {
        if let Some(cmd) = self.effect_to_command(effect) {
            self.command_tx
                .send(cmd)
                .map_err(|_| anyhow::anyhow!("Background task channel closed"))?;
        }
        Ok(())
    }

    /// Store a temporary directory for later use.
    ///
    /// Called after RunGenerator completes to preserve the temp directory
    /// until Serialize runs. The TempDir is moved into the handler.
    ///
    /// # Arguments
    ///
    /// * `temp_dir` - The TempDir to store
    pub fn store_temp_dir(&mut self, temp_dir: tempfile::TempDir) {
        self.current_temp_dir = Some(temp_dir);
    }

    /// Take the stored temporary directory.
    ///
    /// Called by the Serialize effect to retrieve the temp directory
    /// containing generator output. Returns None if no temp directory
    /// is stored (e.g., if RunGenerator hasn't run yet).
    ///
    /// # Returns
    ///
    /// * `Some(TempDir)` - The stored temp directory (consumed)
    /// * `None` - No temp directory stored
    pub fn take_temp_dir(&mut self) -> Option<tempfile::TempDir> {
        self.current_temp_dir.take()
    }

    /// Convert an Effect into an EffectCommand.
    ///
    /// This method extracts the data from the Effect enum and builds the
    /// corresponding EffectCommand for channel transmission. Some effects
    /// (like None, Quit, ShowGeneratorSelection) don't need background
    /// execution and return None.
    ///
    /// # Arguments
    ///
    /// * `effect` - The Effect to convert
    ///
    /// # Returns
    ///
    /// * `Some(EffectCommand)` - Command to send to background
    /// * `None` - Effect handled synchronously or is no-op
    fn effect_to_command(&self, effect: Effect) -> Option<EffectCommand> {
        match effect {
            Effect::None | Effect::Quit => None,

            Effect::CheckSerialization {
                artifact_index,
                artifact_name,
                target,
                target_type,
            } => Some(EffectCommand::CheckSerialization {
                artifact_index,
                artifact_name,
                target,
                target_type: target_type.to_string(),
            }),

            Effect::RunGenerator {
                artifact_index,
                artifact_name,
                target,
                target_type,
                prompts,
            } => Some(EffectCommand::RunGenerator {
                artifact_index,
                artifact_name,
                target,
                target_type: target_type.to_string(),
                prompts,
            }),

            Effect::Serialize {
                artifact_index,
                artifact_name,
                target,
                target_type,
                out_dir: _,
            } => Some(EffectCommand::Serialize {
                artifact_index,
                artifact_name,
                target,
                target_type: target_type.to_string(),
            }),

            Effect::ShowGeneratorSelection { .. } => {
                // This effect is handled synchronously by update() - no background work
                None
            }

            Effect::SharedCheckSerialization {
                artifact_index,
                artifact_name,
                backend_name: _,
                nixos_targets,
                home_targets,
            } => {
                // Convert to EffectCommand format
                let targets: Vec<String> = nixos_targets
                    .iter()
                    .chain(home_targets.iter())
                    .cloned()
                    .collect();
                let target_types: Vec<String> = nixos_targets
                    .iter()
                    .map(|_| "nixos".to_string())
                    .chain(home_targets.iter().map(|_| "home".to_string()))
                    .collect();
                Some(EffectCommand::SharedCheckSerialization {
                    artifact_index,
                    artifact_name,
                    targets,
                    target_types,
                })
            }

            Effect::RunSharedGenerator {
                artifact_index,
                artifact_name,
                generator_path: _,
                prompts,
                nixos_targets,
                home_targets,
                files: _,
            } => Some(EffectCommand::RunSharedGenerator {
                artifact_index,
                artifact_name,
                machine_targets: nixos_targets,
                user_targets: home_targets,
                prompts,
            }),

            Effect::SharedSerialize {
                artifact_index,
                artifact_name,
                backend_name: _,
                out_dir: _,
                nixos_targets,
                home_targets,
            } => Some(EffectCommand::SharedSerialize {
                artifact_index,
                artifact_name,
                machine_targets: nixos_targets,
                user_targets: home_targets,
            }),

            Effect::Batch(effects) => {
                // For batch effects, find the first that needs background execution
                effects.into_iter().find_map(|e| self.effect_to_command(e))
            }
        }
    }

    /// Convert an EffectResult into a Msg for the update loop.
    ///
    /// This method processes the result from the background task and converts
    /// it into the appropriate Msg variant. It also handles temp directory
    /// storage for GeneratorFinished results.
    ///
    /// # Arguments
    ///
    /// * `result` - The EffectResult from the background task
    ///
    /// # Returns
    ///
    /// The Msg variant to feed into the update loop
    pub fn result_to_message(&mut self, result: EffectResult) -> Msg {
        match result {
            EffectResult::CheckSerialization {
                artifact_index,
                needs_generation,
                exists,
                output: _,
            } => {
                Msg::CheckSerializationResult {
                    artifact_index,
                    needs_generation,
                    exists,
                    result: Ok(()),
                    output: None,
                }
            }

            EffectResult::GeneratorFinished {
                artifact_index,
                success,
                output,
                error,
            } => {
                use crate::app::message::GeneratorOutput;
                let result = if success {
                    Ok(GeneratorOutput {
                        stdout_lines: output.stdout_lines,
                        stderr_lines: output.stderr_lines,
                        files_generated: 0, // TODO: Get actual count
                    })
                } else {
                    Err(error.unwrap_or_else(|| "Generator failed".to_string()))
                };
                Msg::GeneratorFinished {
                    artifact_index,
                    result,
                }
            }

            EffectResult::SerializeFinished {
                artifact_index,
                success,
                output,
                error,
            } => {
                use crate::app::message::SerializeOutput;
                let result = if success {
                    Ok(SerializeOutput {
                        stdout_lines: output.stdout_lines,
                        stderr_lines: output.stderr_lines,
                    })
                } else {
                    Err(error.unwrap_or_else(|| "Serialize failed".to_string()))
                };
                Msg::SerializeFinished {
                    artifact_index,
                    result,
                }
            }

            EffectResult::SharedCheckSerialization {
                artifact_index,
                needs_generation,
                exists,
                outputs: _,
            } => {
                // For simplicity, check if any target needs generation or exists
                let any_needs_gen = needs_generation.iter().any(|&b| b);
                let any_exists = exists.iter().any(|&b| b);
                Msg::SharedCheckSerializationResult {
                    artifact_index,
                    needs_generation: any_needs_gen,
                    exists: any_exists,
                    result: Ok(()),
                    output: None,
                }
            }

            EffectResult::SharedGeneratorFinished {
                artifact_index,
                success,
                output,
                error,
            } => {
                use crate::app::message::GeneratorOutput;
                let result = if success {
                    Ok(GeneratorOutput {
                        stdout_lines: output.stdout_lines,
                        stderr_lines: output.stderr_lines,
                        files_generated: 0,
                    })
                } else {
                    Err(error.unwrap_or_else(|| "Shared generator failed".to_string()))
                };
                Msg::SharedGeneratorFinished {
                    artifact_index,
                    result,
                }
            }

            EffectResult::SharedSerializeFinished {
                artifact_index,
                results: _,
            } => {
                // TODO: Aggregate results properly
                use crate::app::message::SerializeOutput;
                Msg::SharedSerializeFinished {
                    artifact_index,
                    result: Ok(SerializeOutput {
                        stdout_lines: vec![],
                        stderr_lines: vec![],
                    }),
                }
            }

            EffectResult::OutputLine {
                artifact_index,
                stream,
                content,
            } => {
                // Streaming output line received during script execution
                Msg::OutputLine {
                    artifact_index,
                    stream: crate::app::model::OutputStream::from(stream),
                    content,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::model::TargetType;

    #[tokio::test]
    async fn test_effect_handler_sends_command() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<EffectCommand>();
        let mut handler = EffectHandler::new(tx);

        // Send a CheckSerialization effect
        let effect = Effect::CheckSerialization {
            artifact_index: 0,
            artifact_name: "test".to_string(),
            target: "machine".to_string(),
            target_type: TargetType::Nixos,
        };

        handler.run_effect(effect).await.unwrap();

        // Verify command was received
        let cmd = rx.recv().await.unwrap();
        match cmd {
            EffectCommand::CheckSerialization { artifact_index, .. } => {
                assert_eq!(artifact_index, 0);
            }
            _ => panic!("Expected CheckSerialization command"),
        }
    }

    #[tokio::test]
    async fn test_effect_handler_store_and_take_temp_dir() {
        let (tx, _) = tokio::sync::mpsc::unbounded_channel::<EffectCommand>();
        let mut handler = EffectHandler::new(tx);

        // Create a temp dir
        let temp_dir = tempfile::TempDir::new().unwrap();
        let path = temp_dir.path().to_path_buf();

        // Store it
        handler.store_temp_dir(temp_dir);
        assert!(handler.current_temp_dir.is_some());

        // Take it
        let taken = handler.take_temp_dir();
        assert!(taken.is_some());
        assert_eq!(taken.unwrap().path(), path);
        assert!(handler.current_temp_dir.is_none());

        // Taking again returns None
        let second_take = handler.take_temp_dir();
        assert!(second_take.is_none());
    }

    #[test]
    fn test_effect_to_command_handles_all_single_variants() {
        let (tx, _) = tokio::sync::mpsc::unbounded_channel::<EffectCommand>();
        let handler = EffectHandler::new(tx);

        // Test None effect
        let cmd = handler.effect_to_command(Effect::None);
        assert!(cmd.is_none());

        // Test Quit effect
        let cmd = handler.effect_to_command(Effect::Quit);
        assert!(cmd.is_none());

        // Test CheckSerialization
        let cmd = handler.effect_to_command(Effect::CheckSerialization {
            artifact_index: 0,
            artifact_name: "test".to_string(),
            target: "machine".to_string(),
            target_type: TargetType::Nixos,
        });
        assert!(matches!(
            cmd,
            Some(EffectCommand::CheckSerialization { .. })
        ));

        // Test RunGenerator
        let cmd = handler.effect_to_command(Effect::RunGenerator {
            artifact_index: 1,
            artifact_name: "test".to_string(),
            target: "machine".to_string(),
            target_type: TargetType::HomeManager,
            prompts: HashMap::new(),
        });
        assert!(matches!(cmd, Some(EffectCommand::RunGenerator { .. })));

        // Test Serialize
        let cmd = handler.effect_to_command(Effect::Serialize {
            artifact_index: 2,
            artifact_name: "test".to_string(),
            target: "machine".to_string(),
            target_type: TargetType::Nixos,
            out_dir: std::path::PathBuf::from("/tmp"),
        });
        assert!(matches!(cmd, Some(EffectCommand::Serialize { .. })));

        // Test ShowGeneratorSelection (returns None - handled synchronously)
        let cmd = handler.effect_to_command(Effect::ShowGeneratorSelection {
            artifact_index: 3,
            artifact_name: "shared".to_string(),
        });
        assert!(cmd.is_none());
    }

    #[test]
    fn test_result_to_message_handles_all_variants() {
        let (tx, _) = tokio::sync::mpsc::unbounded_channel::<EffectCommand>();
        let mut handler = EffectHandler::new(tx);

        // Test CheckSerialization result
        let msg = handler.result_to_message(EffectResult::CheckSerialization {
            artifact_index: 0,
            needs_generation: true,
            exists: false,
            output: ScriptOutput::default(),
        });
        assert!(matches!(msg, Msg::CheckSerializationResult { .. }));

        // Test GeneratorFinished result
        let msg = handler.result_to_message(EffectResult::GeneratorFinished {
            artifact_index: 1,
            success: true,
            output: ScriptOutput::from_message("output"),
            error: None,
        });
        assert!(matches!(msg, Msg::GeneratorFinished { .. }));

        // Test SerializeFinished result
        let msg = handler.result_to_message(EffectResult::SerializeFinished {
            artifact_index: 2,
            success: true,
            output: ScriptOutput::default(),
            error: None,
        });
        assert!(matches!(msg, Msg::SerializeFinished { .. }));

        // Test SharedCheckSerialization result
        let msg = handler.result_to_message(EffectResult::SharedCheckSerialization {
            artifact_index: 3,
            needs_generation: vec![true, false],
            exists: vec![false, true],
            outputs: vec![ScriptOutput::default(), ScriptOutput::default()],
        });
        assert!(matches!(msg, Msg::SharedCheckSerializationResult { .. }));

        // Test SharedGeneratorFinished result
        let msg = handler.result_to_message(EffectResult::SharedGeneratorFinished {
            artifact_index: 4,
            success: true,
            output: ScriptOutput::default(),
            error: None,
        });
        assert!(matches!(msg, Msg::SharedGeneratorFinished { .. }));

        // Test SharedSerializeFinished result
        let msg = handler.result_to_message(EffectResult::SharedSerializeFinished {
            artifact_index: 5,
            results: vec![("target".to_string(), true, ScriptOutput::default())],
        });
        assert!(matches!(msg, Msg::SharedSerializeFinished { .. }));
    }

    #[test]
    fn test_result_to_message_with_failed_generator() {
        let (tx, _) = tokio::sync::mpsc::unbounded_channel::<EffectCommand>();
        let mut handler = EffectHandler::new(tx);

        let msg = handler.result_to_message(EffectResult::GeneratorFinished {
            artifact_index: 0,
            success: false,
            output: ScriptOutput::default(),
            error: Some("Script failed".to_string()),
        });

        match msg {
            Msg::GeneratorFinished { result, .. } => {
                assert!(result.is_err());
                assert_eq!(result.unwrap_err(), "Script failed");
            }
            _ => panic!("Expected GeneratorFinished message"),
        }
    }
}
