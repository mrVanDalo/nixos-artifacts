use crate::app::model::{ArtifactEntry, Model, TargetType};
use crate::app::{Effect, Msg};
use crate::backend::generator::{run_generator_script, verify_generated_files};
use crate::backend::serialization::{run_check_serialization, run_serialize};
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
    ) -> bool {
        let context = target_type.context_str();
        match run_check_serialization(&entry.artifact, target, &self.backend, &self.make, context) {
            Ok(skip) => !skip, // run_check_serialization returns true if we can skip
            Err(_) => true,    // On error, assume we need generation
        }
    }

    fn run_generator_and_store_output(
        &mut self,
        entry: &ArtifactEntry,
        artifact_name: &str,
        target: &str,
        target_type: TargetType,
        prompts: &HashMap<String, String>,
    ) -> Result<(), String> {
        let context = target_type.context_str();

        let prompt_dir = create_temp_dir(Some(&format!("prompt-{}", artifact_name)))
            .context("creating prompt temp dir")
            .map_err(|e| e.to_string())?;

        let out_dir = create_temp_dir(Some(&format!("out-{}", artifact_name)))
            .context("creating output temp dir")
            .map_err(|e| e.to_string())?;

        self.write_prompts_to_directory(prompts, &prompt_dir.path_buf)?;

        run_generator_script(
            &entry.artifact,
            target,
            &self.make.make_base,
            &prompt_dir.path_buf,
            &out_dir.path_buf,
            context,
        )
        .map_err(|e| e.to_string())?;

        verify_generated_files(&entry.artifact, &out_dir.path_buf).map_err(|e| e.to_string())?;

        self.current_out_dir = Some(out_dir.path_buf.clone());
        std::mem::forget(out_dir);

        Ok(())
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
    ) -> Result<(), String> {
        let context = target_type.context_str();

        let out_path = self
            .current_out_dir
            .take()
            .expect("Serialize called without prior RunGenerator");

        let result = run_serialize(
            &entry.artifact,
            &self.backend,
            &out_path,
            target,
            &self.make,
            context,
        );

        let _ = std::fs::remove_dir_all(&out_path);

        result.map_err(|e| e.to_string())
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
                let needs_generation =
                    self.check_if_artifact_needs_generation(entry, &target, target_type);

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
        }
    }
}
