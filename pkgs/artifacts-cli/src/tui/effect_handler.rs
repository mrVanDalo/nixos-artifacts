use crate::app::model::Model;
use crate::app::{Effect, Msg};
use crate::backend::generator::{run_generator_script, verify_generated_files};
use crate::backend::serialization::run_serialize;
use crate::backend::temp_dir::create_temp_dir;
use crate::cli::commands::generate::run_check_serialization;
use crate::config::backend::BackendConfiguration;
use crate::config::make::MakeConfiguration;
use crate::tui::runtime::EffectHandler;
use anyhow::{Context, Result};
use std::path::PathBuf;

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
                let context = target_type.context_str();

                let needs_generation = match run_check_serialization(
                    &entry.artifact,
                    &target,
                    &self.backend,
                    &self.make,
                    context,
                ) {
                    Ok(skip) => !skip, // run_check_serialization returns true if we can skip
                    Err(_) => true,    // On error, assume we need generation
                };

                Ok(vec![Msg::CheckSerializationResult {
                    artifact_index,
                    needs_generation,
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
                let context = target_type.context_str();

                // Create temp directories for prompts and output
                let prompt_dir = create_temp_dir(Some(&format!("prompt-{}", artifact_name)))
                    .context("creating prompt temp dir")?;
                let out_dir = create_temp_dir(Some(&format!("out-{}", artifact_name)))
                    .context("creating output temp dir")?;

                // Write prompts to files
                for (name, value) in &prompts {
                    let path = prompt_dir.path_buf.join(name);
                    std::fs::write(&path, value)
                        .with_context(|| format!("writing prompt file {}", path.display()))?;
                }

                // Run the generator
                let result = run_generator_script(
                    &entry.artifact,
                    &target,
                    &self.make.make_base,
                    &prompt_dir.path_buf,
                    &out_dir.path_buf,
                    context,
                );

                match result {
                    Ok(()) => {
                        // Verify generated files
                        if let Err(e) = verify_generated_files(&entry.artifact, &out_dir.path_buf) {
                            return Ok(vec![Msg::GeneratorFinished {
                                artifact_index,
                                result: Err(e.to_string()),
                            }]);
                        }

                        // Store the output directory for serialization
                        // We need to keep out_dir alive, so we store its path
                        self.current_out_dir = Some(out_dir.path_buf.clone());
                        // Prevent the TempDirGuard from cleaning up
                        std::mem::forget(out_dir);

                        Ok(vec![Msg::GeneratorFinished {
                            artifact_index,
                            result: Ok(()),
                        }])
                    }
                    Err(e) => Ok(vec![Msg::GeneratorFinished {
                        artifact_index,
                        result: Err(e.to_string()),
                    }]),
                }
            }

            Effect::Serialize {
                artifact_index,
                artifact_name: _,
                target,
                target_type,
                out_dir: _,
            } => {
                let entry = &model.artifacts[artifact_index];
                let context = target_type.context_str();

                // Use the stored output directory from the generator
                let out_path = self
                    .current_out_dir
                    .take()
                    .expect("Serialize called without prior RunGenerator");

                let result = run_serialize(
                    &entry.artifact,
                    &self.backend,
                    &out_path,
                    &target,
                    &self.make,
                    context,
                );

                // Clean up the output directory
                let _ = std::fs::remove_dir_all(&out_path);

                match result {
                    Ok(()) => Ok(vec![Msg::SerializeFinished {
                        artifact_index,
                        result: Ok(()),
                    }]),
                    Err(e) => Ok(vec![Msg::SerializeFinished {
                        artifact_index,
                        result: Err(e.to_string()),
                    }]),
                }
            }
        }
    }
}
