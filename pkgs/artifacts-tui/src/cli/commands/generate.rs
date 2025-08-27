use crate::backend::generator::GeneratorManger;
use crate::backend::prompt::PromptManager;
use crate::backend::resolve_path;
use crate::backend::temp_dir::create_temp_dir;
use crate::config::backend::BackendConfiguration;
use crate::config::make::{ArtifactDef, MakeConfiguration};
use anyhow::{Context, Result};
use log::{debug, error, info};
use serde_json::{json, to_string_pretty};
use std::fs;
use std::path::Path;

/// Converts a string into a safe filename by replacing non-alphanumeric characters with underscores.
///
/// # Arguments
///
/// * `name` - The string to sanitize
///
/// # Returns
///
/// A new String containing only ASCII alphanumeric characters and underscores.
fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

fn print_files(artifact: &ArtifactDef, make_base: &Path) {
    if artifact.files.is_empty() {
        return;
    }
    debug!("    files to produce -> {} files", artifact.files.len());
    for f in &artifact.files {
        let resolved = resolve_path(make_base, &f.path);
        debug!(
            "      - {} => {}{}{}",
            f.name,
            resolved.display(),
            f.owner
                .as_ref()
                .map(|o| format!(" owner={}", o))
                .unwrap_or_default(),
            f.group
                .as_ref()
                .map(|g| format!(" group={}", g))
                .unwrap_or_default(),
        );
    }
}

fn run_check_serialization(
    artifact: &ArtifactDef,
    machine: &str,
    backend: &BackendConfiguration,
    make_base: &Path,
) -> Result<bool> {
    let backend_name = &artifact.serialization;
    let backend_entry = backend.get_backend(backend_name)?;
    let artifact_name = sanitize_name(&artifact.name);
    let inputs = create_temp_dir(Some(&format!("inputs-{}", artifact_name)))?;

    for file in &artifact.files {
        let file_name = sanitize_name(&file.name);
        let resolved_path = resolve_path(make_base, &file.path);
        let json_path = inputs.path_buf.join(file_name);

        let text = to_string_pretty(&json!({
            "path": resolved_path,
            "owner": file.owner,
            "group": file.group,
        }))?;

        if let Err(e) = fs::write(&json_path, text) {
            return Err(anyhow::anyhow!(
                "failed to write {}: {}",
                json_path.display(),
                e
            ));
        }
    }

    // Run check_serialization
    let check_path = resolve_path(&backend.base_path, &backend_entry.check_serialization);
    let check_abs = fs::canonicalize(&check_path).unwrap_or_else(|_| check_path.clone());

    debug!(
        "    running check_serialization: env inputs=\"{}\" machine=\"{}\" artifact=\"{}\" {}",
        inputs.path_buf.display(),
        machine,
        artifact.name,
        check_abs.display()
    );

    match std::process::Command::new(&check_abs)
        .env("inputs", &inputs.path_buf)
        .env("machine", machine)
        .env("artifact", &artifact.name)
        .status()
    {
        Ok(status) => {
            if status.success() {
                debug!(
                    "    check_serialization: OK (exit 0) -> skipping generation and serialization for this artifact"
                );
                Ok(true)
            } else {
                debug!(
                    "    check_serialization: failed with status exit status: {} -> continuing with generation",
                    status.code().unwrap_or(0)
                );
                Ok(false)
            }
        }
        Err(error) => Err(anyhow::anyhow!("running check_serialization: {}", error)),
    }
}

fn run_serialize(
    artifact: &ArtifactDef,
    backend: &BackendConfiguration,
    out: &Path,
    machine_name: &str,
) -> Result<()> {
    let backend_name = &artifact.serialization;
    let entry = backend.get_backend(backend_name)?;
    let ser_path = resolve_path(&backend.base_path, &entry.serialize);
    let ser_abs = fs::canonicalize(&ser_path).unwrap_or_else(|_| ser_path.clone());
    let _ = std::process::Command::new("sh")
        .arg(&ser_abs)
        .env("out", out)
        .env("machine", machine_name)
        .env("artifact", &artifact.name)
        .status()
        .map_err(|e| {
            error!("    running serialize: {}", e);
        });
    Ok(())
}

/// Generate plan: read make.json and backend config and print scripts to run.
pub fn run(backend_toml: &Path, make_json: &Path) -> Result<()> {
    let backend = BackendConfiguration::read_backend_config(backend_toml)?;
    let make = MakeConfiguration::read_make_config(make_json)?;

    let prompt_manager = PromptManager::new();
    let generator_manager = GeneratorManger::new();

    for (machine, artifacts) in &make.make_map {
        info!("[generate]");
        info!("machine: {}", machine);
        for artifact in artifacts {
            info!("artifact: {}", artifact.name);
            // Do not print prompts to stdout; inputs are read from stdin and stored in $prompt
            print_files(&artifact, &make.make_base);

            let skip_rest =
                run_check_serialization(&artifact, &machine, &backend, &make.make_base)?;
            if skip_rest {
                continue;
            }

            let prompt_results = match prompt_manager.query_prompts(&artifact) {
                Ok(results) => results,
                Err(e) => {
                    error!("Error could not query all prompts: {}", e);
                    continue;
                }
            };

            let prompt = create_temp_dir(Some(&format!("prompt-{}", artifact.name)))?;
            if let Err(e) = prompt_results.write_prompts_to_files(&prompt.path_buf) {
                error!("Error writing prompt files: {}", e);
            }

            let out = create_temp_dir(Some(&format!("out-{}", artifact.name)))?;

            if let Err(e) = generator_manager.run_generator_script(
                &artifact,
                &make.make_base.clone(),
                &prompt.path_buf,
                &out.path_buf,
            ) {
                // Stop the program with an error if the generator (nix-shell) fails
                return Err(e).context("running generator script");
            }

            // verify if generator created the expected files

            run_serialize(&artifact, &backend, &out.path_buf, &machine)?
        }
    }

    Ok(())
}
