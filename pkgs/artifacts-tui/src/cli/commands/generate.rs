use crate::backend::generator::GeneratorManger;
use crate::backend::prompt::PromptManager;
use crate::backend::resolve_path;
use crate::backend::temp_dir::create_temp_dir;
use crate::config::backend::BackendConfig;
use crate::config::make::ArtifactDef;
use anyhow::{Context, Result};
use serde_json::{from_str as json_from_str, json, to_string_pretty};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

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

fn read_backend_config(backend_toml: &Path) -> Result<(BackendConfig, PathBuf)> {
    let backend_text = fs::read_to_string(backend_toml)
        .with_context(|| format!("reading backend config {}", backend_toml.display()))?;
    let backend_cfg: BackendConfig = toml::from_str(&backend_text)
        .with_context(|| format!("parsing backend config {}", backend_toml.display()))?;
    let backend_base = backend_toml
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    Ok((backend_cfg, backend_base))
}

fn read_make_config(make_json: &Path) -> Result<(HashMap<String, Vec<ArtifactDef>>, PathBuf)> {
    let make_text = fs::read_to_string(make_json)
        .with_context(|| format!("reading make config {}", make_json.display()))?;
    let make_map: HashMap<String, Vec<ArtifactDef>> = json_from_str(&make_text)
        .with_context(|| format!("parsing make config {}", make_json.display()))?;
    let make_base = make_json
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."));
    Ok((make_map, make_base))
}

fn print_files(artifact: &ArtifactDef, make_base: &Path) {
    if artifact.files.is_empty() {
        return;
    }
    println!("    files to produce -> {} files", artifact.files.len());
    for f in &artifact.files {
        let resolved = resolve_path(make_base, &f.path);
        println!(
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

fn maybe_run_check_serialization(
    artifact: &ArtifactDef,
    machine: &str,
    backend_config: &BackendConfig,
    backend_base: &Path,
    make_base: &Path,
    backend_toml: &Path,
) -> Result<bool> {
    let backend_name = &artifact.serialization;
    let Some(entry) = backend_config.get(backend_name) else {
        println!(
            "    WARN: backend '{}' not found in {}",
            backend_name,
            backend_toml.display()
        );
        return Ok(false);
    };

    let artifact_name = sanitize_name(&artifact.name);
    let inputs = create_temp_dir(Some(&format!("inputs-{}", artifact_name)))?;

    let mut skip_rest = false;

    for file in &artifact.files {
        let file_name = sanitize_name(&file.name);
        let resolved_path = resolve_path(make_base, &file.path);
        let json_path = inputs.path_buf.join(file_name);
        match to_string_pretty(&json!({
            "path": resolved_path,
            "owner": file.owner,
            "group": file.group,
        })) {
            Ok(text) => {
                if let Err(e) = fs::write(&json_path, text) {
                    println!("    WARN: failed to write {}: {}", json_path.display(), e);
                }
            }
            Err(e) => println!(
                "    WARN: failed to serialize JSON for {}: {}",
                json_path.display(),
                e
            ),
        }
    }

    // Run check_serialization
    let check_path = resolve_path(backend_base, &entry.check_serialization);
    let check_abs = fs::canonicalize(&check_path).unwrap_or_else(|_| check_path.clone());
    println!(
        "    running check_serialization: env inputs=\"{}\" machine=\"{}\" artifact=\"{}\" {}",
        inputs.path_buf.display(),
        machine,
        artifact.name,
        check_abs.display()
    );
    let status = std::process::Command::new(&check_abs)
        .env("inputs", &inputs.path_buf)
        .env("machine", machine)
        .env("artifact", &artifact.name)
        .status();
    match status {
        Ok(s) => {
            if s.success() {
                println!(
                    "    check_serialization: OK (exit 0) -> skipping generation/serialization for this artifact"
                );
                skip_rest = true;
            } else {
                println!(
                    "    check_serialization: failed with status {} -> continuing with generation",
                    s
                );
            }
        }
        Err(e) => {
            println!(
                "    ERROR running check_serialization: {} -> continuing with generation",
                e
            );
        }
    }

    Ok(skip_rest)
}

fn run_serialize(
    artifact: &ArtifactDef,
    backend_cfg: &BackendConfig,
    backend_base: &Path,
    out: &Path,
    machine: &str,
    backend_toml: &Path,
) {
    let backend_name = &artifact.serialization;
    match backend_cfg.get(backend_name) {
        Some(entry) => {
            let ser_path = resolve_path(backend_base, &entry.serialize);
            let ser_abs = fs::canonicalize(&ser_path).unwrap_or_else(|_| ser_path.clone());
            let _ = std::process::Command::new("sh")
                .arg(&ser_abs)
                .env("out", out)
                .env("machine", machine)
                .env("artifact", &artifact.name)
                .status()
                .map_err(|e| {
                    eprintln!("    ERROR running serialize: {}", e);
                });
        }
        None => {
            println!(
                "    WARN: backend '{}' not found in {}",
                backend_name,
                backend_toml.display()
            );
        }
    }
}

fn process_plan(
    mut make_map: HashMap<String, Vec<ArtifactDef>>,
    make_file_base_path: &Path,
    backend_cfg: &BackendConfig,
    backend_base: &Path,
    backend_toml: &Path,
) -> Result<()> {
    let prompt_manager = PromptManager::new();
    let generator_manager = GeneratorManger::new();

    for (machine, artifacts) in make_map.drain() {
        println!("[generate] machine: {}", machine);
        for artifact in artifacts {
            println!("  - artifact: {}", artifact.name);
            // Do not print prompts to stdout; inputs are read from stdin and stored in $prompt
            print_files(&artifact, make_file_base_path);

            // First, check if we can skip generation/serialization
            let skip_rest = maybe_run_check_serialization(
                &artifact,
                &machine,
                backend_cfg,
                backend_base,
                make_file_base_path,
                backend_toml,
            )?;
            if skip_rest {
                continue;
            }

            let prompt_results = match prompt_manager.query_prompts(&artifact) {
                Ok(results) => results,
                Err(e) => {
                    eprintln!("Error could not query all prompts: {}", e);
                    continue;
                }
            };

            let prompt = create_temp_dir(Some(&format!("prompt-{}", artifact.name)))?;

            if let Err(e) = prompt_results.write_prompts_to_files(&prompt.path_buf) {
                eprintln!("Error writing prompt files: {}", e);
            }

            let out = create_temp_dir(Some(&format!("out-{}", artifact.name)))?;

            if let Err(e) = generator_manager.run_generator_script(
                &artifact,
                make_file_base_path,
                &prompt.path_buf,
                &out.path_buf,
            ) {
                // Stop the program with an error if the generator (nix-shell) fails
                return Err(e).context("running generator script");
            }

            run_serialize(
                &artifact,
                backend_cfg,
                backend_base,
                &out.path_buf,
                &machine,
                backend_toml,
            );
        }
    }
    Ok(())
}

/// Generate plan: read make.json and backend config and print scripts to run.
pub fn run(backend_toml: &Path, make_json: &Path) -> Result<()> {
    // Load backend config (TOML), paths relative to file location
    let (backend_cfg, backend_base) = read_backend_config(backend_toml)?;

    // Load make.json. The format is: { "<machine-name>": [ArtifactDef, ...], ... }
    let (make_map, make_base) = read_make_config(make_json)?;

    // Iterate machines and artifacts; per-artifact temp dirs are prepared inside process_plan
    process_plan(
        make_map,
        &make_base,
        &backend_cfg,
        &backend_base,
        backend_toml,
    )?;

    Ok(())
}
