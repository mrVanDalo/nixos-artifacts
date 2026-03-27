//! Background task that processes effects sequentially.
//!
//! This module provides the background task infrastructure that runs effects
//! asynchronously while the TUI remains responsive. Effects are processed
//! in FIFO order via a tokio task.
//!
//! ## Architecture
//!
//! - **Foreground (TUI thread):** Sends `Effect` messages, receives `Message`
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

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;

use crate::app::effect::Effect;
use crate::app::message::{Message, ScriptOutput};
use crate::app::model::{ArtifactStatus, OutputStream, TargetType};
use crate::backend;
use crate::config::backend::BackendConfiguration;
use crate::config::make::{ArtifactDef, MakeConfiguration};
use crate::logging::{log, log_component};

/// Timeout duration for background task operations (35 seconds)
/// Allows 30s for script execution + 5s buffer for cleanup
pub const BACKGROUND_TASK_TIMEOUT: Duration = Duration::from_secs(35);

/// Result of running a blocking operation with timeout.
///
/// This enum represents all possible outcomes of a spawn_blocking operation
/// wrapped with a timeout, capturing success, operation failure, task panic,
/// or timeout scenarios.
enum TimeoutResult<T> {
    /// Operation completed successfully
    Success(T),
    /// Operation returned an error
    OperationFailed(String),
    /// The spawned task panicked
    TaskPanic(String),
    /// The operation timed out
    Timeout,
}

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
    result_tx: Option<UnboundedSender<Message>>,
    /// Log level to pass to executed scripts
    log_level: crate::logging::LogLevel,
}

impl BackgroundEffectHandler {
    /// Create a new handler with the given configuration.
    ///
    /// The configuration is moved into the handler and owned by the background task.
    pub fn new(
        backend: BackendConfiguration,
        make: MakeConfiguration,
        log_level: crate::logging::LogLevel,
    ) -> Self {
        Self {
            backend,
            make,
            current_output_dir: None,
            current_prompts: None,
            result_tx: None,
            log_level,
        }
    }

    /// Set the result channel sender for streaming output.
    /// This allows the handler to send OutputLine messages during script execution.
    pub fn set_result_sender(&mut self, sender: UnboundedSender<Message>) {
        self.result_tx = Some(sender);
    }

    /// Send an output line to the TUI via the result channel.
    /// Kept for future use - currently output is batched at end of script execution
    /// but we may switch to streaming output in a future phase (see Phase 20: Output Streaming)
    #[allow(dead_code)]
    fn send_output_line(
        &self,
        artifact_index: usize,
        stream: OutputStream,
        content: String,
    ) -> bool {
        if let Some(ref tx) = self.result_tx {
            tx.send(Message::OutputLine {
                artifact_index,
                stream,
                content,
            })
            .is_ok()
        } else {
            false
        }
    }

    // -------------------------------------------------------------------------
    // Utility Methods
    // -------------------------------------------------------------------------

    /// Parse target type string into TargetType enum.
    fn target_type_from_str(target_type: &str, target: &str) -> TargetType {
        if target_type == "home" {
            TargetType::HomeManager {
                username: target.to_string(),
            }
        } else {
            TargetType::NixOS {
                machine: target.to_string(),
            }
        }
    }

    /// Look up an artifact definition in either nixos_map or home_map.
    fn lookup_artifact(&self, target: &str, artifact_name: &str) -> Option<ArtifactDef> {
        self.make
            .nixos_map
            .get(target)
            .and_then(|m| m.get(artifact_name))
            .or_else(|| {
                self.make
                    .home_map
                    .get(target)
                    .and_then(|m| m.get(artifact_name))
            })
            .cloned()
    }

    /// Create a temporary directory with a descriptive error message on failure.
    fn create_temp_dir(purpose: &str) -> Result<tempfile::TempDir, String> {
        tempfile::TempDir::new()
            .map_err(|e| format!("Failed to create {} directory: {}", purpose, e))
    }

    /// Write prompt values to files in the specified directory.
    ///
    /// Each prompt is written to a file named after the prompt key.
    fn write_prompts_to_dir(
        prompts: &HashMap<String, String>,
        dir: &std::path::Path,
    ) -> Result<(), String> {
        for (name, value) in prompts {
            let prompt_file = dir.join(name);
            std::fs::write(&prompt_file, value)
                .map_err(|e| format!("Failed to write prompt file '{}': {}", name, e))?;
        }
        Ok(())
    }

    /// Run a blocking operation with a timeout and proper error handling.
    ///
    /// This wraps spawn_blocking with a timeout and converts all error cases
    /// (operation failure, task panic, timeout) into TimeoutResult variants.
    async fn run_with_timeout<F, T>(
        artifact_name: &str,
        operation_name: &str,
        operation: F,
    ) -> TimeoutResult<T>
    where
        F: FnOnce() -> anyhow::Result<T> + Send + 'static,
        T: Send + 'static,
    {
        let result = timeout(
            BACKGROUND_TASK_TIMEOUT,
            tokio::task::spawn_blocking(operation),
        )
        .await;

        match result {
            Ok(Ok(Ok(value))) => TimeoutResult::Success(value),
            Ok(Ok(Err(e))) => {
                log(&format!(
                    "[ERROR] {} failed for {}: {}",
                    operation_name, artifact_name, e
                ));
                TimeoutResult::OperationFailed(e.to_string())
            }
            Ok(Err(e)) => {
                log(&format!(
                    "[ERROR] {} task panicked for {}: {}",
                    operation_name, artifact_name, e
                ));
                TimeoutResult::TaskPanic(e.to_string())
            }
            Err(_) => {
                log(&format!(
                    "[ERROR] {} timed out for {} after {} seconds",
                    operation_name,
                    artifact_name,
                    BACKGROUND_TASK_TIMEOUT.as_secs()
                ));
                TimeoutResult::Timeout
            }
        }
    }

    /// Create a ScriptOutput for a timeout error.
    fn timeout_output() -> ScriptOutput {
        ScriptOutput::from_message(&format!(
            "Timed out after {} seconds",
            BACKGROUND_TASK_TIMEOUT.as_secs()
        ))
    }

    // -------------------------------------------------------------------------
    // Effect Handler Methods
    // -------------------------------------------------------------------------

    /// Handle CheckSerialization command.
    async fn execute_check_serialization(
        &self,
        artifact_index: usize,
        artifact_name: String,
        target: String,
        target_type: String,
    ) -> Message {
        let artifact = match self.lookup_artifact(&target, &artifact_name) {
            Some(a) => a,
            None => {
                return Message::CheckSerializationResult {
                    artifact_index,
                    status: ArtifactStatus::Failed {
                        error: format!(
                            "Artifact '{}' not found for target '{}'",
                            artifact_name, target
                        ),
                        output: String::new(),
                        retry_available: true,
                    },
                    result: Err(format!(
                        "Artifact '{}' not found for target '{}'",
                        artifact_name, target
                    )),
                };
            }
        };

        let backend = self.backend.clone();
        let make = self.make.clone();
        let target_type_enum = Self::target_type_from_str(&target_type, &target);
        let log_level = self.log_level.as_str().to_string();

        let result = Self::run_with_timeout(&artifact_name, "CheckSerialization", move || {
            backend::serialization::run_check_serialization(
                &artifact,
                &target_type_enum,
                &backend,
                &make,
                &log_level,
            )
        })
        .await;

        match result {
            TimeoutResult::Success(check_result) => {
                let status = if check_result.needs_generation {
                    ArtifactStatus::NeedsGeneration
                } else {
                    ArtifactStatus::UpToDate
                };
                Message::CheckSerializationResult {
                    artifact_index,
                    status,
                    result: Ok(ScriptOutput::from_captured(&check_result.output)),
                }
            }
            TimeoutResult::OperationFailed(e) => Message::CheckSerializationResult {
                artifact_index,
                status: ArtifactStatus::Failed {
                    error: format!("Check failed: {}", e),
                    output: String::new(),
                    retry_available: true,
                },
                result: Err(format!("Check failed: {}", e)),
            },
            TimeoutResult::TaskPanic(e) => Message::CheckSerializationResult {
                artifact_index,
                status: ArtifactStatus::Failed {
                    error: format!("Task panicked: {}", e),
                    output: String::new(),
                    retry_available: true,
                },
                result: Err(format!("Task panicked: {}", e)),
            },
            TimeoutResult::Timeout => Message::CheckSerializationResult {
                artifact_index,
                status: ArtifactStatus::Failed {
                    error: format!(
                        "Timed out after {} seconds",
                        BACKGROUND_TASK_TIMEOUT.as_secs()
                    ),
                    output: String::new(),
                    retry_available: true,
                },
                result: Err(format!(
                    "Timed out after {} seconds",
                    BACKGROUND_TASK_TIMEOUT.as_secs()
                )),
            },
        }
    }

    /// Handle RunGenerator command.
    async fn execute_run_generator(
        &mut self,
        artifact_index: usize,
        artifact_name: String,
        target: String,
        target_type: String,
        prompts: HashMap<String, String>,
    ) -> Message {
        let artifact = match self.lookup_artifact(&target, &artifact_name) {
            Some(a) => a,
            None => {
                return Message::GeneratorFinished {
                    artifact_index,
                    result: Err(format!(
                        "Artifact '{}' not found for target '{}'",
                        artifact_name, target
                    )),
                };
            }
        };

        let temp_dir = match Self::create_temp_dir("output") {
            Ok(dir) => dir,
            Err(e) => {
                return Message::GeneratorFinished {
                    artifact_index,
                    result: Err(e),
                };
            }
        };
        let output_path = temp_dir.path().to_path_buf();

        let prompts_dir = match Self::create_temp_dir("prompts") {
            Ok(dir) => dir,
            Err(e) => {
                return Message::GeneratorFinished {
                    artifact_index,
                    result: Err(e),
                };
            }
        };

        if let Err(e) = Self::write_prompts_to_dir(&prompts, prompts_dir.path()) {
            return Message::GeneratorFinished {
                artifact_index,
                result: Err(e),
            };
        }

        let target_type_enum = Self::target_type_from_str(&target_type, &target);
        let prompts_path = prompts_dir.path().to_path_buf();
        let make_base = self.make.make_base.clone();
        let log_level = self.log_level.as_str().to_string();
        let artifact_for_verify = artifact.clone();
        let output_path_for_verify = output_path.clone();

        let result = Self::run_with_timeout(&artifact_name, "Generator", move || {
            backend::generator::run_generator_script(
                &artifact,
                &target_type_enum,
                &make_base,
                &prompts_path,
                &output_path,
                &log_level,
            )
        })
        .await;

        match result {
            TimeoutResult::Success(output) => {
                let verify_result = backend::generator::verify_generated_files(
                    &artifact_for_verify,
                    &output_path_for_verify,
                );
                match verify_result {
                    Ok(()) => {
                        self.current_output_dir = Some(temp_dir);
                        self.current_prompts = Some(prompts);
                        std::mem::forget(prompts_dir);
                        Message::GeneratorFinished {
                            artifact_index,
                            result: Ok(ScriptOutput::from_captured(&output)),
                        }
                    }
                    Err(e) => {
                        log(&format!(
                            "[ERROR] Generator output verification failed for {}: {}",
                            artifact_name, e
                        ));
                        Message::GeneratorFinished {
                            artifact_index,
                            result: Err(format!("Verification failed: {}", e)),
                        }
                    }
                }
            }
            TimeoutResult::OperationFailed(e) => Message::GeneratorFinished {
                artifact_index,
                result: Err(format!("Generator failed: {}", e)),
            },
            TimeoutResult::TaskPanic(e) => Message::GeneratorFinished {
                artifact_index,
                result: Err(format!("Task panicked: {}", e)),
            },
            TimeoutResult::Timeout => Message::GeneratorFinished {
                artifact_index,
                result: Err(format!(
                    "Timed out after {} seconds",
                    BACKGROUND_TASK_TIMEOUT.as_secs()
                )),
            },
        }
    }

    /// Handle Serialize command.
    async fn execute_serialize(
        &mut self,
        artifact_index: usize,
        artifact_name: String,
        target: String,
        target_type: String,
    ) -> Message {
        let output_dir = match self.current_output_dir.take() {
            Some(dir) => dir,
            None => {
                return Message::SerializeFinished {
                    artifact_index,
                    result: Err("No output directory from generator - RunGenerator must be called before Serialize".to_string()),
                };
            }
        };

        let artifact = match self.lookup_artifact(&target, &artifact_name) {
            Some(a) => a,
            None => {
                return Message::SerializeFinished {
                    artifact_index,
                    result: Err(format!(
                        "Artifact '{}' not found for target '{}'",
                        artifact_name, target
                    )),
                };
            }
        };

        let output_path = output_dir.path().to_path_buf();
        let target_type_enum = Self::target_type_from_str(&target_type, &target);
        let backend = self.backend.clone();
        let make = self.make.clone();
        let log_level = self.log_level.as_str().to_string();

        let result = Self::run_with_timeout(&artifact_name, "Serialize", move || {
            backend::serialization::run_serialize(
                &artifact,
                &backend,
                &output_path,
                &target_type_enum,
                &make,
                &log_level,
            )
        })
        .await;

        match result {
            TimeoutResult::Success(output) => Message::SerializeFinished {
                artifact_index,
                result: Ok(ScriptOutput::from_captured(&output)),
            },
            TimeoutResult::OperationFailed(e) => Message::SerializeFinished {
                artifact_index,
                result: Err(format!("Serialize failed: {}", e)),
            },
            TimeoutResult::TaskPanic(e) => Message::SerializeFinished {
                artifact_index,
                result: Err(format!("Task panicked: {}", e)),
            },
            TimeoutResult::Timeout => Message::SerializeFinished {
                artifact_index,
                result: Err(format!(
                    "Timed out after {} seconds",
                    BACKGROUND_TASK_TIMEOUT.as_secs()
                )),
            },
        }
    }

    /// Handle SharedCheckSerialization command.
    async fn execute_shared_check_serialization(
        &self,
        artifact_index: usize,
        artifact_name: String,
        targets: Vec<String>,
        target_types: Vec<String>,
    ) -> Message {
        let backend = self.backend.clone();
        let make = self.make.clone();

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

        let backend_name = match self.make.get_shared_artifacts().get(&artifact_name) {
            Some(info) => info.backend_name.clone(),
            None => {
                let count = targets.len();
                let status = ArtifactStatus::Failed {
                    error: format!("Shared artifact '{}' not found", artifact_name),
                    output: String::new(),
                    retry_available: true,
                };
                return Message::SharedCheckSerializationResult {
                    artifact_index,
                    statuses: vec![status; count],
                    outputs: vec![
                        ScriptOutput::from_message(&format!(
                            "Shared artifact '{}' not found",
                            artifact_name
                        ));
                        count
                    ],
                };
            }
        };

        let artifact_name_for_closure = artifact_name.clone();
        let log_level = self.log_level.as_str().to_string();
        let result =
            Self::run_with_timeout(&artifact_name, "SharedCheckSerialization", move || {
                backend::serialization::run_shared_check_serialization(
                    &artifact_name_for_closure,
                    &backend_name,
                    &backend,
                    &make,
                    &nixos_targets,
                    &home_targets,
                    &log_level,
                )
            })
            .await;

        let count = targets.len();
        match result {
            TimeoutResult::Success(check_result) => {
                let needs_gen = check_result.needs_generation;
                let status = if needs_gen {
                    ArtifactStatus::NeedsGeneration
                } else {
                    ArtifactStatus::UpToDate
                };
                let statuses = vec![status; count];
                let outputs = vec![ScriptOutput::from_captured(&check_result.output); count];
                Message::SharedCheckSerializationResult {
                    artifact_index,
                    statuses,
                    outputs,
                }
            }
            TimeoutResult::OperationFailed(e) => {
                let status = ArtifactStatus::Failed {
                    error: format!("Check failed: {}", e),
                    output: String::new(),
                    retry_available: true,
                };
                let statuses = vec![status; count];
                let outputs =
                    vec![ScriptOutput::from_message(&format!("Check failed: {}", e)); count];
                Message::SharedCheckSerializationResult {
                    artifact_index,
                    statuses,
                    outputs,
                }
            }
            TimeoutResult::TaskPanic(e) => {
                let status = ArtifactStatus::Failed {
                    error: format!("Task panicked: {}", e),
                    output: String::new(),
                    retry_available: true,
                };
                let statuses = vec![status; count];
                let outputs =
                    vec![ScriptOutput::from_message(&format!("Task panicked: {}", e)); count];
                Message::SharedCheckSerializationResult {
                    artifact_index,
                    statuses,
                    outputs,
                }
            }
            TimeoutResult::Timeout => {
                let status = ArtifactStatus::Failed {
                    error: format!(
                        "Timed out after {} seconds",
                        BACKGROUND_TASK_TIMEOUT.as_secs()
                    ),
                    output: String::new(),
                    retry_available: true,
                };
                let statuses = vec![status; count];
                let outputs = vec![Self::timeout_output(); count];
                Message::SharedCheckSerializationResult {
                    artifact_index,
                    statuses,
                    outputs,
                }
            }
        }
    }

    /// Handle RunSharedGenerator command.
    async fn execute_run_shared_generator(
        &mut self,
        artifact_index: usize,
        artifact_name: String,
        prompts: HashMap<String, String>,
    ) -> Message {
        let shared_info = match self
            .make
            .get_shared_artifacts()
            .get(&artifact_name)
            .cloned()
        {
            Some(info) => info,
            None => {
                return Message::SharedGeneratorFinished {
                    artifact_index,
                    result: Err(format!("Shared artifact '{}' not found", artifact_name)),
                };
            }
        };

        let generator_path = match shared_info.generators.first() {
            Some(gen_info) => gen_info.path.clone(),
            None => {
                return Message::SharedGeneratorFinished {
                    artifact_index,
                    result: Err(format!(
                        "No generators defined for shared artifact '{}'",
                        artifact_name
                    )),
                };
            }
        };

        let files_for_verify = shared_info.files.clone();
        let prompts_for_verify = shared_info.prompts.clone();
        let backend_name_for_verify = shared_info.backend_name.clone();

        let prompts_dir = match Self::create_temp_dir("prompts") {
            Ok(dir) => dir,
            Err(e) => {
                return Message::SharedGeneratorFinished {
                    artifact_index,
                    result: Err(e),
                };
            }
        };

        if let Err(e) = Self::write_prompts_to_dir(&prompts, prompts_dir.path()) {
            return Message::SharedGeneratorFinished {
                artifact_index,
                result: Err(e),
            };
        }

        let out_dir = match Self::create_temp_dir("output") {
            Ok(dir) => dir,
            Err(e) => {
                return Message::SharedGeneratorFinished {
                    artifact_index,
                    result: Err(e),
                };
            }
        };
        let out_path = out_dir.path().to_path_buf();

        let make_base = self.make.make_base.clone();
        let prompts_path = prompts_dir.path().to_path_buf();
        let generator_path_clone = generator_path.clone();
        let artifact_name_for_verify = artifact_name.clone();
        let out_path_for_verify = out_path.clone();
        let log_level = self.log_level.as_str().to_string();

        let result = Self::run_with_timeout(&artifact_name, "SharedGenerator", move || {
            backend::generator::run_generator_script_with_path(
                &generator_path_clone,
                &make_base,
                &prompts_path,
                &out_path,
                &log_level,
            )
        })
        .await;

        match result {
            TimeoutResult::Success(output) => {
                let verify_result = backend::generator::verify_generated_files(
                    &ArtifactDef {
                        name: artifact_name_for_verify.clone(),
                        description: None,
                        files: files_for_verify,
                        prompts: prompts_for_verify,
                        shared: true,
                        serialization: backend_name_for_verify,
                        generator: generator_path,
                    },
                    &out_path_for_verify,
                );
                match verify_result {
                    Ok(()) => {
                        self.current_output_dir = Some(out_dir);
                        self.current_prompts = Some(prompts);
                        std::mem::forget(prompts_dir);
                        Message::SharedGeneratorFinished {
                            artifact_index,
                            result: Ok(ScriptOutput::from_captured(&output)),
                        }
                    }
                    Err(e) => {
                        log(&format!(
                            "[ERROR] Generator output verification failed for {}: {}",
                            artifact_name, e
                        ));
                        Message::SharedGeneratorFinished {
                            artifact_index,
                            result: Err(format!("Verification failed: {}", e)),
                        }
                    }
                }
            }
            TimeoutResult::OperationFailed(e) => Message::SharedGeneratorFinished {
                artifact_index,
                result: Err(format!("Generator failed: {}", e)),
            },
            TimeoutResult::TaskPanic(e) => Message::SharedGeneratorFinished {
                artifact_index,
                result: Err(format!("Task panicked: {}", e)),
            },
            TimeoutResult::Timeout => Message::SharedGeneratorFinished {
                artifact_index,
                result: Err(format!(
                    "Timed out after {} seconds",
                    BACKGROUND_TASK_TIMEOUT.as_secs()
                )),
            },
        }
    }

    /// Handle SharedSerialize command.
    async fn execute_shared_serialize(
        &mut self,
        artifact_index: usize,
        artifact_name: String,
        machine_targets: Vec<String>,
        user_targets: Vec<String>,
    ) -> Message {
        let output_dir = match self.current_output_dir.take() {
            Some(dir) => dir,
            None => {
                let mut results = Vec::new();
                let msg = "No output directory from generator";
                for target in &machine_targets {
                    results.push((target.clone(), false, ScriptOutput::from_message(msg)));
                }
                for target in &user_targets {
                    results.push((target.clone(), false, ScriptOutput::from_message(msg)));
                }
                return Message::SharedSerializeFinished {
                    artifact_index,
                    results,
                };
            }
        };
        let out_path = output_dir.path().to_path_buf();

        let shared_info = match self
            .make
            .get_shared_artifacts()
            .get(&artifact_name)
            .cloned()
        {
            Some(info) => info,
            None => {
                let mut results = Vec::new();
                let msg = format!("Shared artifact '{}' not found", artifact_name);
                for target in &machine_targets {
                    results.push((target.clone(), false, ScriptOutput::from_message(&msg)));
                }
                for target in &user_targets {
                    results.push((target.clone(), false, ScriptOutput::from_message(&msg)));
                }
                return Message::SharedSerializeFinished {
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
        let log_level = self.log_level.as_str().to_string();

        let result = Self::run_with_timeout(&artifact_name, "SharedSerialize", move || {
            backend::serialization::run_shared_serialize(
                &artifact_name_clone,
                &backend_name,
                &backend,
                &out_path,
                &make,
                &nixos_targets,
                &home_targets,
                &log_level,
            )
        })
        .await;

        match result {
            TimeoutResult::Success(output) => {
                let mut results = Vec::new();
                for target in machine_targets {
                    results.push((target, true, ScriptOutput::from_captured(&output)));
                }
                for target in user_targets {
                    results.push((target, true, ScriptOutput::from_captured(&output)));
                }
                Message::SharedSerializeFinished {
                    artifact_index,
                    results,
                }
            }
            TimeoutResult::OperationFailed(e) => {
                let msg = format!("Serialize failed: {}", e);
                let mut results = Vec::new();
                for target in machine_targets {
                    results.push((target, false, ScriptOutput::from_message(&msg)));
                }
                for target in user_targets {
                    results.push((target, false, ScriptOutput::from_message(&msg)));
                }
                Message::SharedSerializeFinished {
                    artifact_index,
                    results,
                }
            }
            TimeoutResult::TaskPanic(e) => {
                let msg = format!("Task panicked: {}", e);
                let mut results = Vec::new();
                for target in machine_targets {
                    results.push((target, false, ScriptOutput::from_message(&msg)));
                }
                for target in user_targets {
                    results.push((target, false, ScriptOutput::from_message(&msg)));
                }
                Message::SharedSerializeFinished {
                    artifact_index,
                    results,
                }
            }
            TimeoutResult::Timeout => {
                let mut results = Vec::new();
                for target in machine_targets {
                    results.push((target, false, Self::timeout_output()));
                }
                for target in user_targets {
                    results.push((target, false, Self::timeout_output()));
                }
                Message::SharedSerializeFinished {
                    artifact_index,
                    results,
                }
            }
        }
    }

    /// Execute a single effect and return the result.
    ///
    /// This is the core effect execution logic that runs in the background.
    /// Uses spawn_blocking for all blocking I/O operations.
    pub async fn execute(&mut self, effect: Effect) -> Message {
        log_component("BACKGROUND", "Starting execution of effect");
        match effect {
            Effect::None | Effect::Quit => Message::CheckSerializationResult {
                artifact_index: 0,
                status: ArtifactStatus::Pending,
                result: Ok(ScriptOutput::default()),
            },

            Effect::CheckSerialization {
                artifact_index,
                artifact_name,
                target_type,
            } => {
                let target = target_type.target_name().unwrap_or("shared").to_string();
                let target_type_str = target_type.to_string();
                self.execute_check_serialization(
                    artifact_index,
                    artifact_name,
                    target,
                    target_type_str,
                )
                .await
            }

            Effect::RunGenerator {
                artifact_index,
                artifact_name,
                target_type,
                prompts,
            } => {
                let target = target_type.target_name().unwrap_or("shared").to_string();
                let target_type_str = target_type.to_string();
                self.execute_run_generator(
                    artifact_index,
                    artifact_name,
                    target,
                    target_type_str,
                    prompts,
                )
                .await
            }

            Effect::Serialize {
                artifact_index,
                artifact_name,
                target_type,
            } => {
                let target = target_type.target_name().unwrap_or("shared").to_string();
                let target_type_str = target_type.to_string();
                self.execute_serialize(artifact_index, artifact_name, target, target_type_str)
                    .await
            }

            Effect::SharedCheckSerialization {
                artifact_index,
                artifact_name,
                nixos_targets,
                home_targets,
            } => {
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
                self.execute_shared_check_serialization(
                    artifact_index,
                    artifact_name,
                    targets,
                    target_types,
                )
                .await
            }

            Effect::RunSharedGenerator {
                artifact_index,
                artifact_name,
                prompts,
            } => {
                self.execute_run_shared_generator(artifact_index, artifact_name, prompts)
                    .await
            }

            Effect::SharedSerialize {
                artifact_index,
                artifact_name,
                nixos_targets,
                home_targets,
            } => {
                self.execute_shared_serialize(
                    artifact_index,
                    artifact_name,
                    nixos_targets,
                    home_targets,
                )
                .await
            }

            Effect::Batch(effects) => {
                // Execute first effect in batch
                if let Some(first) = effects.into_iter().next() {
                    Box::pin(self.execute(first)).await
                } else {
                    Message::CheckSerializationResult {
                        artifact_index: 0,
                        status: ArtifactStatus::Pending,
                        result: Ok(ScriptOutput::default()),
                    }
                }
            }
        }
    }
}

/// Spawn a background task that processes Effects sequentially.
///
/// This function creates the channels and spawns a tokio task that:
/// 1. Creates a BackgroundEffectHandler with the provided configuration
/// 2. Listens for Effect messages on the command channel
/// 3. Executes each effect using the handler
/// 4. Sends Message messages back on the result channel
///
/// Effects are processed in FIFO order. When the TUI closes (drops the result
/// receiver), the background task exits cleanly. The shutdown_token can also
/// be cancelled to initiate graceful shutdown.
///
/// # Arguments
///
/// * `backend` - Backend configuration for effect execution
/// * `make` - Make configuration for effect execution
/// * `log_level` - Log level to pass to executed scripts
/// * `shutdown_token` - CancellationToken for graceful shutdown
///
/// # Returns
///
/// Returns `(tx_cmd, rx_res)` where:
/// - `tx_cmd`: Send `Effect` messages to the background
/// - `rx_res`: Receive `Message` messages from the background
pub fn spawn_background_task(
    backend: BackendConfiguration,
    make: MakeConfiguration,
    log_level: crate::logging::LogLevel,
    shutdown_token: CancellationToken,
) -> (UnboundedSender<Effect>, UnboundedReceiver<Message>) {
    let (tx_cmd, mut rx_cmd) = unbounded_channel::<Effect>();
    let (tx_res, rx_res) = unbounded_channel::<Message>();

    log_component("SPAWN", "About to spawn background task");

    tokio::spawn(async move {
        log_component("BACKGROUND", "Task started");
        let mut handler = BackgroundEffectHandler::new(backend, make, log_level);
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
