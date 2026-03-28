//! Effect handler for executing real backend operations.
//!
//! This module provides the `BackendEffectHandler` which executes real
//! backend operations (check, generator, serialize) for use by the TUI
//! and tests.

use crate::app::effect::Effect;
use crate::app::message::ScriptOutput;
use crate::app::model::{ArtifactEntry, ArtifactStatus, ListEntry, Model, TargetType};
use crate::backend::generator::{run_generator_script, verify_generated_files};
use crate::backend::output_capture::OutputStream;
use crate::backend::serialization::{
    run_check_serialization, run_serialize, run_shared_check_serialization,
};

use crate::config::backend::BackendConfiguration;
use crate::config::make::MakeConfiguration;
use crate::tui::runtime::EffectHandler;
use anyhow::Context;
use std::collections::HashMap;
use std::path::Path;

/// Effect handler that executes real backend operations.
///
/// This connects the TUI to the existing backend infrastructure.
pub struct BackendEffectHandler {
    pub backend: BackendConfiguration,
    pub make: MakeConfiguration,
    /// Temporary output directory for the current generation.
    /// Stored here so Serialize can access the generator output.
    pub current_out_dir: Option<std::path::PathBuf>,
}

impl BackendEffectHandler {
    pub fn new(backend: BackendConfiguration, make: MakeConfiguration) -> Self {
        Self {
            backend,
            make,
            current_out_dir: None,
        }
    }

    fn check_if_artifact_needs_generation(
        &self,
        entry: &ArtifactEntry,
        target_type: &TargetType,
    ) -> (ArtifactStatus, Result<ScriptOutput, String>) {
        match run_check_serialization(
            &entry.artifact,
            target_type,
            &self.backend,
            &self.make,
            "info",
        ) {
            Ok(check_result) => {
                let status = if check_result.needs_generation {
                    ArtifactStatus::NeedsGeneration
                } else {
                    ArtifactStatus::UpToDate
                };
                (
                    status,
                    Ok(ScriptOutput::from_captured(&check_result.output)),
                )
            }
            Err(e) => (
                ArtifactStatus::Failed {
                    error: e.to_string(),
                    output: String::new(),
                    retry_available: true,
                },
                Err(e.to_string()),
            ),
        }
    }

    fn run_generator_and_store_output(
        &mut self,
        entry: &ArtifactEntry,
        target_type: &TargetType,
        prompts: &HashMap<String, String>,
    ) -> Result<(Vec<String>, Vec<String>, usize), String> {
        let prompt_dir = tempfile::TempDir::new()
            .context("creating prompt temp dir")
            .map_err(|e| e.to_string())?;

        let out_dir = tempfile::TempDir::new()
            .context("creating output temp dir")
            .map_err(|e| e.to_string())?;

        self.write_prompts_to_directory(prompts, prompt_dir.path())?;

        let captured = run_generator_script(
            &entry.artifact,
            target_type,
            &self.make.make_base,
            prompt_dir.path(),
            out_dir.path(),
            "info",
        )
        .map_err(|e| e.to_string())?;

        verify_generated_files(&entry.artifact, out_dir.path()).map_err(|e| e.to_string())?;

        let files_generated = entry.artifact.files.len();

        self.current_out_dir = Some(out_dir.path().to_path_buf());
        std::mem::forget(out_dir);

        let (stdout_lines, stderr_lines) = split_captured_output(&captured);

        Ok((stdout_lines, stderr_lines, files_generated))
    }

    fn write_prompts_to_directory(
        &self,
        prompts: &HashMap<String, String>,
        prompt_dir: &Path,
    ) -> Result<(), String> {
        for (name, value) in prompts {
            let path = prompt_dir.join(name);
            std::fs::write(&path, value)
                .with_context(|| format!("writing prompt file {}", path.display()))
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    fn serialize_generated_output_to_backend(
        &mut self,
        entry: &ArtifactEntry,
        target_type: &TargetType,
    ) -> Result<(Vec<String>, Vec<String>), String> {
        let out_path = self
            .current_out_dir
            .take()
            .expect("Serialize called without prior RunGenerator");

        let captured = run_serialize(
            &entry.artifact,
            &self.backend,
            &out_path,
            target_type,
            &self.make,
            "info",
        )
        .map_err(|e| e.to_string())?;

        let _ = std::fs::remove_dir_all(&out_path);

        let (stdout_lines, stderr_lines) = split_captured_output(&captured);

        Ok((stdout_lines, stderr_lines))
    }
}

/// Split captured output into separate stdout and stderr line vectors
fn split_captured_output(
    captured: &crate::backend::output_capture::CapturedOutput,
) -> (Vec<String>, Vec<String>) {
    let mut stdout_lines = Vec::new();
    let mut stderr_lines = Vec::new();

    for line in &captured.lines {
        match line.stream {
            OutputStream::Stdout => stdout_lines.push(line.content.clone()),
            OutputStream::Stderr => stderr_lines.push(line.content.clone()),
        }
    }

    (stdout_lines, stderr_lines)
}

impl EffectHandler for BackendEffectHandler {
    fn execute(
        &mut self,
        effect: Effect,
        model: &Model,
    ) -> anyhow::Result<Vec<crate::app::message::Message>> {
        match effect {
            Effect::None | Effect::Quit | Effect::Batch(_) => Ok(vec![]),

            Effect::CheckSerialization {
                artifact_index,
                artifact_name: _,
                target_type,
            } => {
                let entry = match &model.entries[artifact_index] {
                    ListEntry::Single(entry) => entry.clone(),
                    ListEntry::Shared(_) => {
                        return Ok(vec![
                            crate::app::message::Message::CheckSerializationResult {
                                artifact_index,
                                status: ArtifactStatus::Failed {
                                    error: "CheckSerialization effect called on shared artifact"
                                        .to_string(),
                                    output: String::new(),
                                    retry_available: true,
                                },
                                result: Err("CheckSerialization effect called on shared artifact"
                                    .to_string()),
                            },
                        ]);
                    }
                };
                let (status, result) =
                    self.check_if_artifact_needs_generation(&entry, &target_type);

                Ok(vec![
                    crate::app::message::Message::CheckSerializationResult {
                        artifact_index,
                        status,
                        result,
                    },
                ])
            }

            Effect::RunGenerator {
                artifact_index,
                artifact_name: _,
                target_type,
                prompts,
            } => {
                let entry = match &model.entries[artifact_index] {
                    ListEntry::Single(entry) => entry.clone(),
                    ListEntry::Shared(_) => {
                        return Ok(vec![crate::app::message::Message::GeneratorFinished {
                            artifact_index,
                            result: Err("RunGenerator effect called on shared artifact".to_string()),
                        }]);
                    }
                };
                let result = self.run_generator_and_store_output(&entry, &target_type, &prompts);

                Ok(vec![crate::app::message::Message::GeneratorFinished {
                    artifact_index,
                    result: result.map(|(stdout, stderr, _files)| ScriptOutput {
                        stdout_lines: stdout,
                        stderr_lines: stderr,
                    }),
                }])
            }

            Effect::Serialize {
                artifact_index,
                artifact_name: _,
                target_type,
            } => {
                let entry = match &model.entries[artifact_index] {
                    ListEntry::Single(entry) => entry.clone(),
                    ListEntry::Shared(_) => {
                        return Ok(vec![crate::app::message::Message::SerializeFinished {
                            artifact_index,
                            result: Err("Serialize effect called on shared artifact".to_string()),
                        }]);
                    }
                };
                let result = self.serialize_generated_output_to_backend(&entry, &target_type);

                Ok(vec![crate::app::message::Message::SerializeFinished {
                    artifact_index,
                    result: result.map(|(stdout, stderr)| ScriptOutput {
                        stdout_lines: stdout,
                        stderr_lines: stderr,
                    }),
                }])
            }

            Effect::SharedCheckSerialization {
                artifact_index,
                artifact_name,
                nixos_targets,
                home_targets,
            } => {
                // Get the artifact from the model to find its serialization backend
                let artifact_entry = match &model.entries[artifact_index] {
                    ListEntry::Shared(entry) => entry.clone(),
                    ListEntry::Single(_) => {
                        return Ok(vec![
                            crate::app::message::Message::SharedCheckSerializationResult {
                                artifact_index,
                                statuses: vec![ArtifactStatus::Failed {
                                    error: "SharedCheckSerialization called on non-shared artifact"
                                        .to_string(),
                                    output: String::new(),
                                    retry_available: true,
                                }],
                                outputs: vec![],
                            },
                        ]);
                    }
                };

                let backend_name = artifact_entry.info.backend_name.clone();

                let result = run_shared_check_serialization(
                    &artifact_name,
                    &backend_name,
                    &self.backend,
                    &self.make,
                    &nixos_targets,
                    &home_targets,
                    "info",
                );

                match result {
                    Ok(check_result) => {
                        let status = if check_result.needs_generation {
                            ArtifactStatus::NeedsGeneration
                        } else {
                            ArtifactStatus::UpToDate
                        };
                        Ok(vec![
                            crate::app::message::Message::SharedCheckSerializationResult {
                                artifact_index,
                                statuses: vec![status],
                                outputs: vec![ScriptOutput::from_captured(&check_result.output)],
                            },
                        ])
                    }
                    Err(e) => Ok(vec![
                        crate::app::message::Message::SharedCheckSerializationResult {
                            artifact_index,
                            statuses: vec![ArtifactStatus::Failed {
                                error: e.to_string(),
                                output: String::new(),
                                retry_available: true,
                            }],
                            outputs: vec![],
                        },
                    ]),
                }
            }

            Effect::RunSharedGenerator { .. } | Effect::SharedSerialize { .. } => {
                // These effects require the full async runtime with channels
                // For the synchronous EffectHandler trait, we return an error
                Ok(vec![crate::app::message::Message::GeneratorFinished {
                    artifact_index: 0,
                    result: Err("Shared artifact effects require async runtime".to_string()),
                }])
            }
        }
    }
}
