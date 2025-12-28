use crate::backend::helpers::resolve_path;
use crate::backend::temp_dir::create_temp_dir;
use crate::config::backend::BackendConfiguration;
use crate::config::make::{ArtifactDef, MakeConfiguration};
use anyhow::{Context, Result};
use log::debug;
use serde_json::{json, to_string_pretty};
use std::fs;
use std::path::Path;

/// Run the serialize script for a generated artifact.
///
/// This function resolves the serialize script path from the backend
/// configuration and invokes it with the appropriate environment variables:
/// - out: path to the generator output directory
/// - machine: the machine name
/// - artifact: the artifact name
pub fn run_serialize(
    artifact: &ArtifactDef,
    backend: &BackendConfiguration,
    out: &Path,
    target_name: &str,
    make: &MakeConfiguration,
    context: &str,
) -> Result<()> {
    let backend_name = &artifact.serialization;
    let entry = backend.get_backend(backend_name)?;
    let ser_path = resolve_path(&backend.base_path, &entry.serialize);
    let ser_abs = fs::canonicalize(&ser_path).unwrap_or_else(|_| ser_path.clone());
    println!("💾 serialize secrets");

    // Create config file for the selected backend and machine
    let config_dir = create_temp_dir(Some("config"))?;
    let config_file = config_dir.path_buf.join("config.json");
    let config_value = make
        .get_backend_config_for(target_name, backend_name)
        .map(|m| serde_json::to_value(m).unwrap_or(serde_json::json!({})))
        .unwrap_or(serde_json::json!({}));
    let config_text = to_string_pretty(&config_value)?;
    fs::write(&config_file, &config_text)
        .with_context(|| format!("writing {}", config_file.display()))?;

    let mut cmd = std::process::Command::new("sh");
    cmd.arg(&ser_abs)
        .env("out", out)
        .env("config", &config_file)
        .env("artifact_context", context)
        .env("artifact", &artifact.name);

    if context == "homemanager" {
        cmd.env("username", target_name);
    } else {
        cmd.env("machine", target_name);
    }

    cmd.status()
        .with_context(|| format!("running serialize {}", ser_abs.display()))?;
    Ok(())
}

/// Check if serialization is up to date.
///
/// Returns Ok(true) if the artifact can be skipped (already serialized),
/// Ok(false) if regeneration is needed.
pub fn run_check_serialization(
    artifact: &ArtifactDef,
    target: &str,
    backend: &BackendConfiguration,
    make: &MakeConfiguration,
    context: &str,
) -> Result<bool> {
    let backend_name = &artifact.serialization;
    let backend_entry = backend.get_backend(backend_name)?;

    let inputs = create_temp_dir(Some(&format!("inputs-{}", artifact.name)))?;

    // Prepare backend config file
    let config_dir = create_temp_dir(Some(&format!("config-{}", artifact.name)))?;
    let config_file = config_dir.path_buf.join("config.json");
    let config_value = make
        .get_backend_config_for(target, backend_name)
        .map(|m| serde_json::to_value(m).unwrap_or(json!({})))
        .unwrap_or(json!({}));
    let config_text = to_string_pretty(&config_value)?;
    fs::write(&config_file, &config_text)
        .with_context(|| format!("writing {}", config_file.display()))?;

    // Write input files
    for file in artifact.files.values() {
        let resolved_path = file
            .path
            .as_ref()
            .map(|path| resolve_path(&make.make_base, path));
        let json_path = inputs.path_buf.join(&file.name);

        let text = to_string_pretty(&json!({
            "path": resolved_path,
            "owner": file.owner,
            "group": file.group,
        }))?;

        fs::write(&json_path, text).with_context(|| format!("writing {}", json_path.display()))?;
    }

    // Run check_serialization script
    let check_path = resolve_path(&backend.base_path, &backend_entry.check_serialization);
    let check_abs = fs::canonicalize(&check_path).unwrap_or_else(|_| check_path.clone());

    let target_label = if context == "homemanager" {
        "username"
    } else {
        "machine"
    };
    debug!(
        "running check_serialization: env inputs=\"{}\" {}=\"{}\" artifact=\"{}\" {}",
        inputs.path_buf.display(),
        target_label,
        target,
        artifact.name,
        check_abs.display()
    );

    let mut cmd = std::process::Command::new(&check_abs);
    cmd.env("inputs", &inputs.path_buf)
        .env("config", &config_file)
        .env("artifact_context", context)
        .env("artifact", &artifact.name);

    if context == "homemanager" {
        cmd.env("username", target);
    } else {
        cmd.env("machine", target);
    }

    let status = cmd
        .status()
        .with_context(|| format!("running check_serialization {}", check_abs.display()))?;

    if status.success() {
        debug!("check_serialization: OK -> skipping generation");
        Ok(true)
    } else {
        debug!("check_serialization: needs generation");
        Ok(false)
    }
}
