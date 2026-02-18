//! Background task that processes effects sequentially.
//!
//! This module provides the background task infrastructure that runs effects
//! asynchronously while the TUI remains responsive. Effects are processed
//! in FIFO order via a tokio task.
//!
//! ## Architecture
//!
//! - **Foreground (TUI thread):** Sends `EffectCommand` messages, receives `EffectResult`
//! - **Background (tokio task):** Receives commands, executes effects, sends results
//! - **Communication:** Unbounded channels (mpsc) for async message passing
//!
//! ## Design Decisions
//!
//! - **Single background task:** One task processes all effects sequentially (not per-effect)
//! - **FIFO ordering:** Commands processed in order received via `while let Some()`
//! - **Graceful shutdown:** Background exits cleanly when TUI drops result channel
//! - **No shared mutable state:** Handler is owned by background task

use std::collections::HashMap;
use std::time::Duration;

use tokio::io::AsyncBufReadExt;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;

use crate::backend;
use crate::config::backend::BackendConfiguration;
use crate::config::make::{ArtifactDef, MakeConfiguration};
use crate::logging::{log, log_component};
use crate::tui::channels::{EffectCommand, EffectResult, OutputStream, ScriptOutput};

/// Timeout duration for background task operations (35 seconds)
/// Allows 30s for script execution + 5s buffer for cleanup
pub const BACKGROUND_TASK_TIMEOUT: Duration = Duration::from_secs(35);

/// Handler that executes effects in the background task.
///
/// This struct is created once and lives in the background task.
/// It holds immutable copies of configuration needed for effect execution.
pub struct BackgroundEffectHandler {
    backend: BackendConfiguration,
    make: MakeConfiguration,
    /// Temporary output directory from generator, preserved for serialize
    current_output_dir: Option<tempfile::TempDir>,
    /// Collected prompts for generator execution
    current_prompts: Option<HashMap<String, String>>,
    /// Channel sender for streaming output during script execution
    result_tx: Option<UnboundedSender<EffectResult>>,
}

impl BackgroundEffectHandler {
    /// Create a new handler with the given configuration.
    ///
    /// The configuration is moved into the handler and owned by the background task.
    pub fn new(backend: BackendConfiguration, make: MakeConfiguration) -> Self {
        Self {
            backend,
            make,
            current_output_dir: None,
            current_prompts: None,
            result_tx: None,
        }
    }

    /// Set the result channel sender for streaming output.
    /// This allows the handler to send OutputLine messages during script execution.
    pub fn set_result_sender(&mut self, sender: UnboundedSender<EffectResult>) {
        self.result_tx = Some(sender);
    }

    /// Send a streaming output line to the foreground.
    /// Returns true if the line was sent successfully.
    fn send_output_line(&self, artifact_index: usize, stream: OutputStream, content: String) -> bool {
        if let Some(ref tx) = self.result_tx {
            tx.send(EffectResult::OutputLine {
                artifact_index,
                stream,
                content,
            }).is_ok()
        } else {
            false
        }
    }

    /// Execute a single effect command and return the result.
    ///
    /// This is the core effect execution logic that runs in the background.
    /// Uses spawn_blocking for all blocking I/O operations.
    pub async fn execute(&mut self, cmd: EffectCommand) -> EffectResult {
        log_component("BACKGROUND", &format!("Starting execution of command"));
        match cmd {
            EffectCommand::CheckSerialization {
                artifact_index,
                artifact_name,
                target,
                target_type,
            } => {
                // Clone configuration for use in spawn_blocking
                let backend = self.backend.clone();
                let make = self.make.clone();
                let artifact_name_for_error = artifact_name.clone();
                let target_for_error = target.clone();

                // Clone values for timeout error handling
                let artifact_name_for_timeout = artifact_name.clone();

                // Spawn blocking task to execute check_serialization with timeout
                let result = timeout(
                    BACKGROUND_TASK_TIMEOUT,
                    tokio::task::spawn_blocking(move || {
                        // Look up the artifact definition
                        let artifact = make
                            .nixos_map
                            .get(&target)
                            .and_then(|m| m.get(&artifact_name))
                            .or_else(|| {
                                make.home_map
                                    .get(&target)
                                    .and_then(|m| m.get(&artifact_name))
                            })
                            .cloned()
                            .ok_or_else(|| {
                                anyhow::anyhow!(
                                    "Artifact '{}' not found for target '{}'",
                                    artifact_name,
                                    target
                                )
                            })?;

                        let context = if target_type == "home" {
                            "homemanager"
                        } else {
                            "nixos"
                        };

                        backend::serialization::run_check_serialization(
                            &artifact, &target, &backend, &make, context,
                        )
                    }),
                )
                .await;

                match result {
                    Ok(Ok(Ok(check_result))) => EffectResult::CheckSerialization {
                        artifact_index,
                        needs_generation: check_result.needs_generation,
                        output: ScriptOutput::from_captured(&check_result.output),
                    },
                    Ok(Ok(Err(e))) => {
                        // Fail open - assume generation needed on error
                        log(&format!(
                            "[WARN] CheckSerialization failed for {}: {}",
                            artifact_name_for_error, e
                        ));
                        EffectResult::CheckSerialization {
                            artifact_index,
                            needs_generation: true,
                            output: ScriptOutput::from_message(&format!("Check failed: {}", e)),
                        }
                    }
                    Ok(Err(e)) => {
                        // Task panicked
                        log(&format!(
                            "[ERROR] CheckSerialization task panicked for {}: {}",
                            artifact_name_for_error, e
                        ));
                        EffectResult::CheckSerialization {
                            artifact_index,
                            needs_generation: true,
                            output: ScriptOutput::from_message(&format!("Task panicked: {}", e)),
                        }
                    }
                    Err(_) => {
                        // Timeout occurred
                        log(&format!(
                            "[ERROR] CheckSerialization timed out for {} after {} seconds",
                            artifact_name_for_timeout,
                            BACKGROUND_TASK_TIMEOUT.as_secs()
                        ));
                        EffectResult::CheckSerialization {
                            artifact_index,
                            needs_generation: true,
                            output: ScriptOutput::from_message(&format!(
                                "Timed out after {} seconds",
                                BACKGROUND_TASK_TIMEOUT.as_secs()
                            )),
                        }
                    }
                }
            }

            EffectCommand::RunGenerator {
                artifact_index,
                artifact_name,
                target,
                target_type,
                prompts,
            } => {
                // Clone data for use in spawn_blocking
                let make = self.make.clone();
                let artifact_name_clone = artifact_name.clone();
                let target_clone = target.clone();

                // Create temporary directory for generator output
                let temp_dir = match tempfile::TempDir::new() {
                    Ok(dir) => dir,
                    Err(e) => {
                        return EffectResult::GeneratorFinished {
                            artifact_index,
                            success: false,
                            output: ScriptOutput::default(),
                            error: Some(format!("Failed to create temp directory: {}", e)),
                        };
                    }
                };
                let output_path = temp_dir.path().to_path_buf();

                // Look up artifact definition
                let artifact = match make
                    .nixos_map
                    .get(&target)
                    .and_then(|m| m.get(&artifact_name))
                    .or_else(|| {
                        make.home_map
                            .get(&target)
                            .and_then(|m| m.get(&artifact_name))
                    })
                    .cloned()
                {
                    Some(art) => art,
                    None => {
                        return EffectResult::GeneratorFinished {
                            artifact_index,
                            success: false,
                            output: ScriptOutput::default(),
                            error: Some(format!(
                                "Artifact '{}' not found for target '{}'",
                                artifact_name, target
                            )),
                        };
                    }
                };

                // Create prompts directory
                let prompts_dir = match tempfile::TempDir::new() {
                    Ok(dir) => dir,
                    Err(e) => {
                        return EffectResult::GeneratorFinished {
                            artifact_index,
                            success: false,
                            output: ScriptOutput::default(),
                            error: Some(format!("Failed to create prompts directory: {}", e)),
                        };
                    }
                };

                // Write prompts to files
                for (name, value) in &prompts {
                    let prompt_file = prompts_dir.path().join(name);
                    if let Err(e) = std::fs::write(&prompt_file, value) {
                        return EffectResult::GeneratorFinished {
                            artifact_index,
                            success: false,
                            output: ScriptOutput::default(),
                            error: Some(format!("Failed to write prompt file: {}", e)),
                        };
                    }
                }

                let context = if target_type == "home" {
                    "homemanager"
                } else {
                    "nixos"
                };
                let prompts_path = prompts_dir.path().to_path_buf();
                let make_base = make.make_base.clone();

                // Clone values for use in spawn_blocking
                let artifact_for_spawn = artifact.clone();
                let output_path_for_verify = output_path.clone();

                // Clone values for timeout error handling
                let artifact_name_for_timeout = artifact_name_clone.clone();

                // Spawn blocking task to execute generator with timeout
                let result = timeout(
                    BACKGROUND_TASK_TIMEOUT,
                    tokio::task::spawn_blocking(move || {
                        backend::generator::run_generator_script(
                            &artifact_for_spawn,
                            &target,
                            &make_base,
                            &prompts_path,
                            &output_path,
                            context,
                        )
                    }),
                )
                .await;

                match result {
                    Ok(Ok(Ok(output))) => {
                        // Verify generated files
                        let verify_result = backend::generator::verify_generated_files(
                            &artifact,
                            &output_path_for_verify,
                        );
                        match verify_result {
                            Ok(()) => {
                                // Store temp directory to keep it alive for Serialize
                                self.current_output_dir = Some(temp_dir);
                                self.current_prompts = Some(prompts);
                                EffectResult::GeneratorFinished {
                                    artifact_index,
                                    success: true,
                                    output: ScriptOutput::from_captured(&output),
                                    error: None,
                                }
                            }
                            Err(e) => {
                                log(&format!(
                                    "[ERROR] Generator output verification failed for {}: {}",
                                    artifact_name_clone, e
                                ));
                                EffectResult::GeneratorFinished {
                                    artifact_index,
                                    success: false,
                                    output: ScriptOutput::from_captured(&output),
                                    error: Some(format!("Verification failed: {}", e)),
                                }
                            }
                        }
                    }
                    Ok(Ok(Err(e))) => {
                        log(&format!(
                            "[ERROR] Generator failed for {}: {}",
                            artifact_name_clone, e
                        ));
                        EffectResult::GeneratorFinished {
                            artifact_index,
                            success: false,
                            output: ScriptOutput::default(),
                            error: Some(format!("Generator failed: {}", e)),
                        }
                    }
                    Ok(Err(e)) => {
                        log(&format!(
                            "[ERROR] Generator task panicked for {}: {}",
                            target_clone, e
                        ));
                        EffectResult::GeneratorFinished {
                            artifact_index,
                            success: false,
                            output: ScriptOutput::default(),
                            error: Some(format!("Task panicked: {}", e)),
                        }
                    }
                    Err(_) => {
                        // Timeout occurred
                        log(&format!(
                            "[ERROR] Generator timed out for {} after {} seconds",
                            artifact_name_for_timeout,
                            BACKGROUND_TASK_TIMEOUT.as_secs()
                        ));
                        EffectResult::GeneratorFinished {
                            artifact_index,
                            success: false,
                            output: ScriptOutput::default(),
                            error: Some(format!(
                                "Timed out after {} seconds",
                                BACKGROUND_TASK_TIMEOUT.as_secs()
                            )),
                        }
                    }
                }
            }

            EffectCommand::Serialize {
                artifact_index,
                artifact_name,
                target,
                target_type,
            } => {
                // Get the output directory from the previous RunGenerator
                let output_dir = match self.current_output_dir.take() {
                    Some(dir) => dir,
                    None => {
                        return EffectResult::SerializeFinished {
                            artifact_index,
                            success: false,
                            output: ScriptOutput::from_message(
                                "No output directory from generator - RunGenerator must be called before Serialize"
                            ),
                            error: Some("No output directory from generator - RunGenerator must be called before Serialize".to_string()),
                        };
                    }
                };

                // Look up artifact definition to get files and backend name
                let artifact = match self
                    .make
                    .nixos_map
                    .get(&target)
                    .and_then(|m| m.get(&artifact_name))
                    .or_else(|| {
                        self.make
                            .home_map
                            .get(&target)
                            .and_then(|m| m.get(&artifact_name))
                    })
                    .cloned()
                {
                    Some(art) => art,
                    None => {
                        return EffectResult::SerializeFinished {
                            artifact_index,
                            success: false,
                            output: ScriptOutput::from_message(&format!(
                                "Artifact '{}' not found for target '{}'",
                                artifact_name, target
                            )),
                            error: Some(format!(
                                "Artifact '{}' not found for target '{}'",
                                artifact_name, target
                            )),
                        };
                    }
                };

                // Extract files from artifact (for future use if needed)
                let _files: Vec<(String, crate::config::make::FileDef)> = artifact
                    .files
                    .iter()
                    .map(|(name, def)| (name.clone(), def.clone()))
                    .collect();

                let _backend_name = artifact.serialization.clone();
                let context = if target_type == "home" {
                    "homemanager"
                } else {
                    "nixos"
                };
                let backend = self.backend.clone();
                let make = self.make.clone();
                let artifact_name_for_error = artifact_name.clone();
                let output_path = output_dir.path().to_path_buf();

                // Clone values for timeout error handling
                let artifact_name_for_timeout = artifact_name.clone();

                // Spawn blocking task to execute serialization with timeout
                let result = timeout(
                    BACKGROUND_TASK_TIMEOUT,
                    tokio::task::spawn_blocking(move || {
                        backend::serialization::run_serialize(
                            &artifact,
                            &backend,
                            &output_path,
                            &target,
                            &make,
                            &context,
                        )
                    }),
                )
                .await;

                match result {
                    Ok(Ok(Ok(output))) => {
                        // Temp directory will be automatically cleaned up when dropped
                        EffectResult::SerializeFinished {
                            artifact_index,
                            success: true,
                            output: ScriptOutput::from_captured(&output),
                            error: None,
                        }
                    }
                    Ok(Ok(Err(e))) => {
                        log(&format!(
                            "[ERROR] Serialize failed for {}: {}",
                            artifact_name_for_error, e
                        ));
                        EffectResult::SerializeFinished {
                            artifact_index,
                            success: false,
                            output: ScriptOutput::from_message(&format!("Serialize failed: {}", e)),
                            error: Some(format!("Serialize failed: {}", e)),
                        }
                    }
                    Ok(Err(e)) => {
                        log(&format!(
                            "[ERROR] Serialize task panicked for {}: {}",
                            artifact_name_for_error, e
                        ));
                        EffectResult::SerializeFinished {
                            artifact_index,
                            success: false,
                            output: ScriptOutput::from_message(&format!("Task panicked: {}", e)),
                            error: Some(format!("Task panicked: {}", e)),
                        }
                    }
                    Err(_) => {
                        // Timeout occurred
                        log(&format!(
                            "[ERROR] Serialize timed out for {} after {} seconds",
                            artifact_name_for_timeout,
                            BACKGROUND_TASK_TIMEOUT.as_secs()
                        ));
                        EffectResult::SerializeFinished {
                            artifact_index,
                            success: false,
                            output: ScriptOutput::from_message(&format!(
                                "Timed out after {} seconds",
                                BACKGROUND_TASK_TIMEOUT.as_secs()
                            )),
                            error: Some(format!(
                                "Timed out after {} seconds",
                                BACKGROUND_TASK_TIMEOUT.as_secs()
                            )),
                        }
                    }
                }
            }

            EffectCommand::SharedCheckSerialization {
                artifact_index,
                artifact_name,
                targets,
                target_types,
            } => {
                // Implement actual shared check_serialization
                let backend = self.backend.clone();
                let make = self.make.clone();
                let artifact_name_for_error = artifact_name.clone();

                // Split targets into nixos and home based on target_types
                let mut nixos_targets = Vec::new();
                let mut home_targets = Vec::new();
                for (i, target) in targets.iter().enumerate() {
                    let target_type = target_types.get(i).map(|s| s.as_str()).unwrap_or("nixos");
                    if target_type == "home" {
                        home_targets.push(target.clone());
                    } else {
                        nixos_targets.push(target.clone());
                    }
                }

                // Get backend name from shared artifacts
                let backend_name = match self.make.get_shared_artifacts().get(&artifact_name) {
                    Some(info) => info.backend_name.clone(),
                    None => {
                        return EffectResult::SharedCheckSerialization {
                            artifact_index,
                            needs_generation: vec![true; targets.len()],
                            outputs: vec![ScriptOutput::from_message(
                                &format!("Shared artifact '{}' not found", artifact_name)
                            )],
                        };
                    }
                };

                // Clone values for timeout error handling
                let artifact_name_for_timeout = artifact_name.clone();

                // Spawn blocking task to execute shared check_serialization with timeout
                let result = timeout(
                    BACKGROUND_TASK_TIMEOUT,
                    tokio::task::spawn_blocking(move || {
                        backend::serialization::run_shared_check_serialization(
                            &artifact_name,
                            &backend_name,
                            &backend,
                            &make,
                            &nixos_targets,
                            &home_targets,
                        )
                    }),
                )
                .await;

                match result {
                    Ok(Ok(Ok(check_result))) => {
                        // Shared artifacts are all-or-nothing - if any target needs generation, all do
                        let needs_gen = check_result.needs_generation;
                        // All targets get the same result (shared artifacts are atomic)
                        let needs_generation = vec![needs_gen; targets.len()];
                        let outputs: Vec<ScriptOutput> =
                            vec![ScriptOutput::from_captured(&check_result.output); targets.len()];
                        EffectResult::SharedCheckSerialization {
                            artifact_index,
                            needs_generation,
                            outputs,
                        }
                    }
                    Ok(Ok(Err(e))) => {
                        // Fail open - assume generation needed on error
                        log(&format!(
                            "[WARN] SharedCheckSerialization failed for {}: {}",
                            artifact_name_for_error, e
                        ));
                        let needs_generation = vec![true; targets.len()];
                        let outputs: Vec<ScriptOutput> =
                            vec![ScriptOutput::from_message(&format!("Check failed: {}", e)); targets.len()];
                        EffectResult::SharedCheckSerialization {
                            artifact_index,
                            needs_generation,
                            outputs,
                        }
                    }
                    Ok(Err(e)) => {
                        log(&format!(
                            "[ERROR] SharedCheckSerialization task panicked for {}: {}",
                            artifact_name_for_error, e
                        ));
                        let needs_generation = vec![true; targets.len()];
                        let outputs: Vec<ScriptOutput> =
                            vec![ScriptOutput::from_message(&format!("Task panicked: {}", e)); targets.len()];
                        EffectResult::SharedCheckSerialization {
                            artifact_index,
                            needs_generation,
                            outputs,
                        }
                    }
                    Err(_) => {
                        // Timeout occurred
                        log(&format!(
                            "[ERROR] SharedCheckSerialization timed out for {} after {} seconds",
                            artifact_name_for_timeout,
                            BACKGROUND_TASK_TIMEOUT.as_secs()
                        ));
                        let needs_generation = vec![true; targets.len()];
                        let outputs: Vec<ScriptOutput> = vec![
                            ScriptOutput::from_message(&format!(
                                "Timed out after {} seconds",
                                BACKGROUND_TASK_TIMEOUT.as_secs()
                            ));
                            targets.len()
                        ];
                        EffectResult::SharedCheckSerialization {
                            artifact_index,
                            needs_generation,
                            outputs,
                        }
                    }
                }
            }

            EffectCommand::RunSharedGenerator {
                artifact_index,
                artifact_name,
                machine_targets,
                user_targets: _,
                prompts,
            } => {
                // Implement actual shared generator execution
                let make = self.make.clone();
                let artifact_name_clone = artifact_name.clone();

                // Get shared artifact info to find generator path - must clone to avoid borrow issues
                let shared_info = match self
                    .make
                    .get_shared_artifacts()
                    .get(&artifact_name)
                    .cloned()
                {
                    Some(info) => info,
                    None => {
                        return EffectResult::SharedGeneratorFinished {
                            artifact_index,
                            success: false,
                            output: ScriptOutput::default(),
                            error: Some(format!("Shared artifact '{}' not found", artifact_name)),
                        };
                    }
                };

                // Get generator path - use the first generator's path
                let generator_path = match shared_info.generators.first() {
                    Some(r#gen) => r#gen.path.clone(),
                    None => {
                        return EffectResult::SharedGeneratorFinished {
                            artifact_index,
                            success: false,
                            output: ScriptOutput::default(),
                            error: Some(format!(
                                "No generators defined for shared artifact '{}'",
                                artifact_name
                            )),
                        };
                    }
                };

                // Get files for verification (clone to avoid borrow issues)
                let files_for_verify = shared_info.files.clone();
                let prompts_for_verify = shared_info.prompts.clone();
                let backend_name_for_verify = shared_info.backend_name.clone();

                // Create temporary directory for prompts
                let prompts_dir = match tempfile::TempDir::new() {
                    Ok(dir) => dir,
                    Err(e) => {
                        return EffectResult::SharedGeneratorFinished {
                            artifact_index,
                            success: false,
                            output: ScriptOutput::default(),
                            error: Some(format!("Failed to create prompts directory: {}", e)),
                        };
                    }
                };

                // Write prompts to files
                for (name, value) in &prompts {
                    let prompt_file = prompts_dir.path().join(name);
                    if let Err(e) = std::fs::write(&prompt_file, value) {
                        return EffectResult::SharedGeneratorFinished {
                            artifact_index,
                            success: false,
                            output: ScriptOutput::default(),
                            error: Some(format!("Failed to write prompt file: {}", e)),
                        };
                    }
                }

                // Create temporary output directory
                let out_dir = match tempfile::TempDir::new() {
                    Ok(dir) => dir,
                    Err(e) => {
                        return EffectResult::SharedGeneratorFinished {
                            artifact_index,
                            success: false,
                            output: ScriptOutput::default(),
                            error: Some(format!("Failed to create output directory: {}", e)),
                        };
                    }
                };
                let out_path = out_dir.path().to_path_buf();

                let make_base = make.make_base.clone();
                let prompts_path = prompts_dir.path().to_path_buf();
                let generator_path_clone = generator_path.clone();
                let out_path_clone = out_path.clone();

                // Clone values for timeout error handling
                let artifact_name_for_timeout = artifact_name_clone.clone();

                // Spawn blocking task to execute generator with timeout
                let result = timeout(
                    BACKGROUND_TASK_TIMEOUT,
                    tokio::task::spawn_blocking(move || {
                        backend::generator::run_generator_script_with_path(
                            &generator_path_clone,
                            &make_base,
                            &prompts_path,
                            &out_path_clone,
                        )
                    }),
                )
                .await;

                match result {
                    Ok(Ok(Ok(output))) => {
                        // Verify generated files using the artifact's file definitions
                        let verify_result = backend::generator::verify_generated_files(
                            &ArtifactDef {
                                name: artifact_name_clone.clone(),
                                files: files_for_verify,
                                prompts: prompts_for_verify,
                                shared: true,
                                serialization: backend_name_for_verify,
                                generator: generator_path,
                            },
                            &out_path,
                        );
                        match verify_result {
                            Ok(()) => {
                                // Store output directory to keep it alive for Serialize phase
                                self.current_output_dir = Some(out_dir);
                                self.current_prompts = Some(prompts);
                                // Keep prompts dir alive too
                                std::mem::forget(prompts_dir);
                                EffectResult::SharedGeneratorFinished {
                                    artifact_index,
                                    success: true,
                                    output: ScriptOutput::from_captured(&output),
                                    error: None,
                                }
                            }
                            Err(e) => {
                                log(&format!(
                                    "[ERROR] Generator output verification failed for {}: {}",
                                    artifact_name_clone, e
                                ));
                                EffectResult::SharedGeneratorFinished {
                                    artifact_index,
                                    success: false,
                                    output: ScriptOutput::from_captured(&output),
                                    error: Some(format!("Verification failed: {}", e)),
                                }
                            }
                        }
                    }
                    Ok(Ok(Err(e))) => {
                        log(&format!(
                            "[ERROR] Shared generator failed for {}: {}",
                            artifact_name_clone, e
                        ));
                        EffectResult::SharedGeneratorFinished {
                            artifact_index,
                            success: false,
                            output: ScriptOutput::default(),
                            error: Some(format!("Generator failed: {}", e)),
                        }
                    }
                    Ok(Err(e)) => {
                        log(&format!(
                            "[ERROR] Shared generator task panicked for {}: {}",
                            artifact_name_clone, e
                        ));
                        EffectResult::SharedGeneratorFinished {
                            artifact_index,
                            success: false,
                            output: ScriptOutput::default(),
                            error: Some(format!("Task panicked: {}", e)),
                        }
                    }
                    Err(_) => {
                        // Timeout occurred
                        log(&format!(
                            "[ERROR] Shared generator timed out for {} after {} seconds",
                            artifact_name_for_timeout,
                            BACKGROUND_TASK_TIMEOUT.as_secs()
                        ));
                        EffectResult::SharedGeneratorFinished {
                            artifact_index,
                            success: false,
                            output: ScriptOutput::default(),
                            error: Some(format!(
                                "Timed out after {} seconds",
                                BACKGROUND_TASK_TIMEOUT.as_secs()
                            )),
                        }
                    }
                }
            }

            EffectCommand::SharedSerialize {
                artifact_index,
                artifact_name,
                machine_targets,
                user_targets,
            } => {
                // Implement actual shared serialization
                // Get output directory from previous generator run
                let output_dir = match self.current_output_dir.take() {
                    Some(dir) => dir,
                    None => {
                        let mut results = Vec::new();
                        for target in &machine_targets {
                            results.push((
                                target.clone(),
                                false,
                                ScriptOutput::from_message("No output directory from generator"),
                            ));
                        }
                        for target in &user_targets {
                            results.push((
                                target.clone(),
                                false,
                                ScriptOutput::from_message("No output directory from generator"),
                            ));
                        }
                        return EffectResult::SharedSerializeFinished {
                            artifact_index,
                            results,
                        };
                    }
                };
                let out_path = output_dir.path().to_path_buf();

                // Get shared artifact info for backend name
                let shared_info = match self
                    .make
                    .get_shared_artifacts()
                    .get(&artifact_name)
                    .cloned()
                {
                    Some(info) => info,
                    None => {
                        let mut results = Vec::new();
                        for target in &machine_targets {
                            results.push((
                                target.clone(),
                                false,
                                ScriptOutput::from_message(&format!("Shared artifact '{}' not found", artifact_name)),
                            ));
                        }
                        for target in &user_targets {
                            results.push((
                                target.clone(),
                                false,
                                ScriptOutput::from_message(&format!("Shared artifact '{}' not found", artifact_name)),
                            ));
                        }
                        return EffectResult::SharedSerializeFinished {
                            artifact_index,
                            results,
                        };
                    }
                };

                let backend = self.backend.clone();
                let make = self.make.clone();
                let backend_name = shared_info.backend_name.clone();
                let artifact_name_clone = artifact_name.clone();

                let nixos_targets = machine_targets.clone();
                let home_targets = user_targets.clone();

                // Clone values for timeout error handling
                let artifact_name_for_timeout = artifact_name.clone();

                // Spawn blocking task to execute shared serialization with timeout
                let result = timeout(
                    BACKGROUND_TASK_TIMEOUT,
                    tokio::task::spawn_blocking(move || {
                        backend::serialization::run_shared_serialize(
                            &artifact_name_clone,
                            &backend_name,
                            &backend,
                            &out_path,
                            &make,
                            &nixos_targets,
                            &home_targets,
                        )
                    }),
                )
                .await;

                match result {
                    Ok(Ok(Ok(output))) => {
                        // Shared serialization is atomic - all succeed or all fail
                        let mut results = Vec::new();
                        for target in machine_targets {
                            results.push((target, true, ScriptOutput::from_captured(&output)));
                        }
                        for target in user_targets {
                            results.push((target, true, ScriptOutput::from_captured(&output)));
                        }
                        EffectResult::SharedSerializeFinished {
                            artifact_index,
                            results,
                        }
                    }
                    Ok(Ok(Err(e))) => {
                        log(&format!(
                            "[ERROR] SharedSerialize failed for {}: {}",
                            artifact_name, e
                        ));
                        let mut results = Vec::new();
                        for target in machine_targets {
                            results.push((target, false, ScriptOutput::from_message(&format!("Serialize failed: {}", e))));
                        }
                        for target in user_targets {
                            results.push((target, false, ScriptOutput::from_message(&format!("Serialize failed: {}", e))));
                        }
                        EffectResult::SharedSerializeFinished {
                            artifact_index,
                            results,
                        }
                    }
                    Ok(Err(e)) => {
                        log(&format!(
                            "[ERROR] SharedSerialize task panicked for {}: {}",
                            artifact_name, e
                        ));
                        let mut results = Vec::new();
                        for target in machine_targets {
                            results.push((target, false, ScriptOutput::from_message(&format!("Task panicked: {}", e))));
                        }
                        for target in user_targets {
                            results.push((target, false, ScriptOutput::from_message(&format!("Task panicked: {}", e))));
                        }
                        EffectResult::SharedSerializeFinished {
                            artifact_index,
                            results,
                        }
                    }
                    Err(_) => {
                        // Timeout occurred
                        log(&format!(
                            "[ERROR] SharedSerialize timed out for {} after {} seconds",
                            artifact_name_for_timeout,
                            BACKGROUND_TASK_TIMEOUT.as_secs()
                        ));
                        let mut results = Vec::new();
                        for target in machine_targets {
                            results.push((
                                target,
                                false,
                                ScriptOutput::from_message(
                                    &format!(
                                        "Timed out after {} seconds",
                                        BACKGROUND_TASK_TIMEOUT.as_secs()
                                    )
                                ),
                            ));
                        }
                        for target in user_targets {
                            results.push((
                                target,
                                false,
                                ScriptOutput::from_message(
                                    &format!(
                                        "Timed out after {} seconds",
                                        BACKGROUND_TASK_TIMEOUT.as_secs()
                                    )
                                ),
                            ));
                        }
                        EffectResult::SharedSerializeFinished {
                            artifact_index,
                            results,
                        }
                    }
                }
            }
        }
    }
}

/// Spawn a background task that processes EffectCommands sequentially.
///
/// This function creates the channels and spawns a tokio task that:
/// 1. Creates a BackgroundEffectHandler with the provided configuration
/// 2. Listens for EffectCommand messages on the command channel
/// 3. Executes each command using the handler
/// 4. Sends EffectResult messages back on the result channel
///
/// Effects are processed in FIFO order. When the TUI closes (drops the result
/// receiver), the background task exits cleanly. The shutdown_token can also
/// be cancelled to initiate graceful shutdown.
///
/// # Arguments
///
/// * `backend` - Backend configuration for effect execution
/// * `make` - Make configuration for effect execution
/// * `shutdown_token` - CancellationToken for graceful shutdown
///
/// # Returns
///
/// Returns `(tx_cmd, rx_res)` where:
/// - `tx_cmd`: Send `EffectCommand` messages to the background
/// - `rx_res`: Receive `EffectResult` messages from the background
///
/// # Example
///
/// ```rust
/// let (tx_cmd, rx_res) = spawn_background_task(backend, make);
///
/// // Send a command
/// tx_cmd.send(EffectCommand::CheckSerialization { ... });
///
/// // Receive the result
/// let result = rx_res.recv().await;
/// ```
pub fn spawn_background_task(
    backend: BackendConfiguration,
    make: MakeConfiguration,
    shutdown_token: CancellationToken,
) -> (
    UnboundedSender<EffectCommand>,
    UnboundedReceiver<EffectResult>,
) {
    let (tx_cmd, mut rx_cmd) = unbounded_channel::<EffectCommand>();
    let (tx_res, rx_res) = unbounded_channel::<EffectResult>();

    log_component("SPAWN", "About to spawn background task");

    tokio::spawn(async move {
        log_component("BACKGROUND", "Task started");
        let mut handler = BackgroundEffectHandler::new(backend, make);
        handler.set_result_sender(tx_res.clone());
        log_component("BACKGROUND", "Handler initialized");

        // Process effects sequentially with graceful shutdown support
        loop {
            tokio::select! {
                // Check for shutdown signal first
                _ = shutdown_token.cancelled() => {
                    log_component("BACKGROUND", "Shutdown requested, finishing current work");
                    // Process any remaining commands in queue before exiting
                    while let Ok(cmd) = rx_cmd.try_recv() {
                        log_component("BACKGROUND", "Processing queued command before shutdown");
                        let result = handler.execute(cmd).await;
                        let _ = tx_res.send(result); // Best effort
                    }
                    log_component("BACKGROUND", "Exiting cleanly");
                    break;
                }

                // Process next command
                Some(cmd) = rx_cmd.recv() => {
                    log_component("BACKGROUND", &format!("Received command: {:?}", cmd));
                    log_component("BACKGROUND", "Executing command...");
                    let result = handler.execute(cmd).await;
                    log_component("BACKGROUND", "Command complete");

                    // Send result back; if TUI closed, exit cleanly
                    log_component("BACKGROUND", "Sending result");
                    if tx_res.send(result).is_err() {
                        log_component("BACKGROUND", "TUI closed (channel closed), exiting");
                        break;
                    }
                    log_component("BACKGROUND", "Result sent successfully");
                }

                // Channel closed
                else => {
                    log_component("BACKGROUND", "Command channel closed, exiting");
                    break;
                }
            }
        }

        // Handler is dropped here, cleaning up temp directories
        log_component("BACKGROUND", "Task complete, handler dropped");
    });

    log_component("SPAWN", "Background task spawned");

    (tx_cmd, rx_res)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_spawn_background_task_creates_channels() {
        // Create minimal configurations for testing
        // These will be default/empty since we can't easily construct full configs
        let backend_config = BackendConfiguration {
            config: std::collections::HashMap::new(),
            base_path: std::path::PathBuf::from("."),
            backend_toml: std::path::PathBuf::from("./test.toml"),
        };

        let make_config = MakeConfiguration {
            nixos_map: std::collections::BTreeMap::new(),
            home_map: std::collections::BTreeMap::new(),
            nixos_config: std::collections::BTreeMap::new(),
            home_config: std::collections::BTreeMap::new(),
            make_base: std::path::PathBuf::from("."),
            make_json: std::path::PathBuf::from("./test.json"),
        };

        let shutdown_token = CancellationToken::new();
        let (tx, mut rx) = spawn_background_task(backend_config, make_config, shutdown_token);

        // Send a command and verify we can receive the result
        let cmd = EffectCommand::CheckSerialization {
            artifact_index: 0,
            artifact_name: "test".to_string(),
            target: "machine".to_string(),
            target_type: "nixos".to_string(),
        };

        tx.send(cmd).unwrap();

        // Receive the result (stub returns immediately)
        let result = rx.recv().await;
        assert!(result.is_some(), "Should receive a result");

        // Verify the result has the correct artifact_index
        match result.unwrap() {
            EffectResult::CheckSerialization { artifact_index, .. } => {
                assert_eq!(artifact_index, 0, "artifact_index should match");
            }
            _ => panic!("Expected CheckSerialization result"),
        }
    }

    #[tokio::test]
    async fn test_fifo_ordering() {
        // Send multiple commands and verify they're processed in order
        let backend_config = BackendConfiguration {
            config: std::collections::HashMap::new(),
            base_path: std::path::PathBuf::from("."),
            backend_toml: std::path::PathBuf::from("./test.toml"),
        };

        let make_config = MakeConfiguration {
            nixos_map: std::collections::BTreeMap::new(),
            home_map: std::collections::BTreeMap::new(),
            nixos_config: std::collections::BTreeMap::new(),
            home_config: std::collections::BTreeMap::new(),
            make_base: std::path::PathBuf::from("."),
            make_json: std::path::PathBuf::from("./test.json"),
        };

        let shutdown_token = CancellationToken::new();
        let (tx, mut rx) = spawn_background_task(backend_config, make_config, shutdown_token);

        // Send 3 commands with sequential indices
        for i in 0..3 {
            tx.send(EffectCommand::CheckSerialization {
                artifact_index: i,
                artifact_name: format!("artifact{}", i),
                target: "machine".to_string(),
                target_type: "nixos".to_string(),
            })
            .unwrap();
        }

        // Receive in order and verify FIFO
        for i in 0..3 {
            let result = rx.recv().await.expect("Should receive result");
            match result {
                EffectResult::CheckSerialization { artifact_index, .. } => {
                    assert_eq!(artifact_index, i, "FIFO order violated at index {}", i);
                }
                _ => panic!("Unexpected result variant"),
            }
        }
    }

    #[tokio::test]
    async fn test_graceful_exit_on_channel_close() {
        // Verify background task exits cleanly when result channel is dropped
        let backend_config = BackendConfiguration {
            config: std::collections::HashMap::new(),
            base_path: std::path::PathBuf::from("."),
            backend_toml: std::path::PathBuf::from("./test.toml"),
        };

        let make_config = MakeConfiguration {
            nixos_map: std::collections::BTreeMap::new(),
            home_map: std::collections::BTreeMap::new(),
            nixos_config: std::collections::BTreeMap::new(),
            home_config: std::collections::BTreeMap::new(),
            make_base: std::path::PathBuf::from("."),
            make_json: std::path::PathBuf::from("./test.json"),
        };

        let shutdown_token = CancellationToken::new();
        let (tx, rx) = spawn_background_task(backend_config, make_config, shutdown_token);

        // Drop the receiver (simulating TUI closing)
        drop(rx);

        // Send a command - this should not panic, background task will exit
        // Note: The command may or may not be sent successfully depending on timing
        let _ = tx.send(EffectCommand::CheckSerialization {
            artifact_index: 0,
            artifact_name: "test".to_string(),
            target: "machine".to_string(),
            target_type: "nixos".to_string(),
        });

        // If we get here without panicking, the test passes
        // The background task should have exited cleanly
    }
}
