use crate::backend::helpers::resolve_path;
use crate::backend::temp_dir::create_temp_dir;
use crate::config::backend::BackendConfiguration;
use crate::config::make::{ArtifactDef, MakeConfiguration};
use anyhow::{Context, Result};
use serde_json::to_string_pretty;
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
    machine_name: &str,
    make: &MakeConfiguration,
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
        .machine_config
        .get(machine_name)
        .and_then(|per_machine| per_machine.get(backend_name))
        .map(|m| serde_json::to_value(m).unwrap_or(serde_json::json!({})))
        .unwrap_or(serde_json::json!({}));
    let config_text = to_string_pretty(&config_value)?;
    fs::write(&config_file, &config_text)
        .with_context(|| format!("writing {}", config_file.display()))?;

    std::process::Command::new("sh")
        .arg(&ser_abs)
        .env("out", out)
        .env("config", &config_file)
        .env("machine", machine_name)
        .env("artifact", &artifact.name)
        .status()
        .with_context(|| format!("running serialize {}", ser_abs.display()))?;
    Ok(())
}
