use crate::app::message::{CheckOutput, GeneratorOutput, SerializeOutput};
use crate::app::model::{ArtifactEntry, Model, TargetType};
use crate::app::{Effect, Msg};
use crate::backend::generator::{run_generator_script, verify_generated_files};
use crate::backend::output_capture::{CapturedOutput, OutputStream};
use crate::backend::serialization::{
    run_check_serialization, run_serialize, run_shared_check_serialization,
};
use crate::backend::temp_dir::create_temp_dir;
use crate::config::backend::BackendConfiguration;
use crate::config::make::MakeConfiguration;
use crate::tui::runtime::EffectHandler;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Effect handler that executes real backend operations.
/// This connects the TUI to the existing backend infrastructure.
pub struct BackendEffectHandler {
    pub backend: BackendConfiguration,
    pub make: MakeConfiguration,
    /// Temporary output directory for the current generation.
    /// Stored here so Serialize can access the generator output.
    current_out_dir: Option<PathBuf>,
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
        target: &str,
        target_type: TargetType,
    ) -> (bool, bool, Result<(), String>, Option<CheckOutput>) {
        let context = target_type.context_str();
        match run_check_serialization(&entry.artifact, target, &self.backend, &self.make, context) {
            Ok(check_result) => {
                let (stdout_lines, stderr_lines) = split_captured_output(&check_result.output);
                let output = CheckOutput {
                    stdout_lines,
                    stderr_lines,
                };
                // Determine exists from check script output
                // If exit success (exit code 0), artifact exists and is up to date
                // If exit failure, check output for "EXISTS" keyword
                let exists = if check_result.output.exit_success {
                    true
                } else {
                    // Check if "EXISTS" appears in any output line
                    check_result
                        .output
                        .lines
                        .iter()
                        .any(|line| line.content.contains("EXISTS"))
                };
                let needs_generation = check_result.needs_generation;
                (needs_generation, exists, Ok(()), Some(output))
            }
            Err(e) => (true, false, Err(e.to_string()), None),
        }
    }

    fn run_generator_and_store_output(
        &mut self,
        entry: &ArtifactEntry,
        artifact_name: &str,
        target: &str,
        target_type: TargetType,
        prompts: &HashMap<String, String>,
    ) -> Result<GeneratorOutput, String> {
        let context = target_type.context_str();

        let prompt_dir = create_temp_dir(Some(&format!("prompt-{}", artifact_name)))
            .context("creating prompt temp dir")
            .map_err(|e| e.to_string())?;

        let out_dir = create_temp_dir(Some(&format!("out-{}", artifact_name)))
            .context("creating output temp dir")
            .map_err(|e| e.to_string())?;

        self.write_prompts_to_directory(prompts, &prompt_dir.path_buf)?;

        let captured = run_generator_script(
            &entry.artifact,
            target,
            &self.make.make_base,
            &prompt_dir.path_buf,
            &out_dir.path_buf,
            context,
        )
        .map_err(|e| e.to_string())?;

        verify_generated_files(&entry.artifact, &out_dir.path_buf).map_err(|e| e.to_string())?;

        let files_generated = entry.artifact.files.len();

        self.current_out_dir = Some(out_dir.path_buf.clone());
        std::mem::forget(out_dir);

        let (stdout_lines, stderr_lines) = split_captured_output(&captured);

        Ok(GeneratorOutput {
            stdout_lines,
            stderr_lines,
            files_generated,
        })
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
        target: &str,
        target_type: TargetType,
    ) -> Result<SerializeOutput, String> {
        let context = target_type.context_str();

        let out_path = self
            .current_out_dir
            .take()
            .expect("Serialize called without prior RunGenerator");

        let captured = run_serialize(
            &entry.artifact,
            &self.backend,
            &out_path,
            target,
            &self.make,
            context,
        )
        .map_err(|e| e.to_string())?;

        let _ = std::fs::remove_dir_all(&out_path);

        let (stdout_lines, stderr_lines) = split_captured_output(&captured);

        Ok(SerializeOutput {
            stdout_lines,
            stderr_lines,
        })
    }
}

/// Split captured output into separate stdout and stderr line vectors
fn split_captured_output(captured: &CapturedOutput) -> (Vec<String>, Vec<String>) {
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
    fn execute(&mut self, effect: Effect, model: &Model) -> Result<Vec<Msg>> {
        match effect {
            Effect::None | Effect::Quit | Effect::Batch(_) => Ok(vec![]),

            Effect::CheckSerialization {
                artifact_index,
                artifact_name: _,
                target,
                target_type,
            } => {
                let entry = &model.artifacts[artifact_index];
                let (needs_generation, exists, result, output) =
                    self.check_if_artifact_needs_generation(entry, &target, target_type);

                Ok(vec![Msg::CheckSerializationResult {
                    artifact_index,
                    needs_generation,
                    exists,
                    result,
                    output,
                }])
            }

            Effect::RunGenerator {
                artifact_index,
                artifact_name,
                target,
                target_type,
                prompts,
            } => {
                let entry = &model.artifacts[artifact_index];
                let result = self.run_generator_and_store_output(
                    entry,
                    &artifact_name,
                    &target,
                    target_type,
                    &prompts,
                );

                Ok(vec![Msg::GeneratorFinished {
                    artifact_index,
                    result,
                }])
            }

            Effect::Serialize {
                artifact_index,
                artifact_name: _,
                target,
                target_type,
                out_dir: _,
            } => {
                let entry = &model.artifacts[artifact_index];
                let result =
                    self.serialize_generated_output_to_backend(entry, &target, target_type);

                Ok(vec![Msg::SerializeFinished {
                    artifact_index,
                    result,
                }])
            }

            Effect::SharedCheckSerialization {
                artifact_index,
                artifact_name,
                backend_name,
                nixos_targets,
                home_targets,
            } => {
                let result = run_shared_check_serialization(
                    &artifact_name,
                    &backend_name,
                    &self.backend,
                    &self.make,
                    &nixos_targets,
                    &home_targets,
                );

                match result {
                    Ok(check_result) => {
                        let (stdout_lines, stderr_lines) =
                            split_captured_output(&check_result.output);
                        let output = CheckOutput {
                            stdout_lines,
                            stderr_lines,
                        };
                        // Determine exists from check script output
                        let exists = if check_result.output.exit_success {
                            true
                        } else {
                            check_result
                                .output
                                .lines
                                .iter()
                                .any(|line| line.content.contains("EXISTS"))
                        };
                        Ok(vec![Msg::SharedCheckSerializationResult {
                            artifact_index,
                            needs_generation: check_result.needs_generation,
                            exists,
                            result: Ok(()),
                            output: Some(output),
                        }])
                    }
                    Err(e) => Ok(vec![Msg::SharedCheckSerializationResult {
                        artifact_index,
                        needs_generation: true,
                        exists: false,
                        result: Err(e.to_string()),
                        output: None,
                    }]),
                }
            }
        }
    }
}
