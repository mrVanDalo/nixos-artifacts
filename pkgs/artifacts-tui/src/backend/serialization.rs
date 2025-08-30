use crate::backend::helpers::resolve_path;
use crate::config::backend::BackendConfiguration;
use crate::config::make::ArtifactDef;
use anyhow::{Context, Result};
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
) -> Result<()> {
    let backend_name = &artifact.serialization;
    let entry = backend.get_backend(backend_name)?;
    let ser_path = resolve_path(&backend.base_path, &entry.serialize);
    let ser_abs = fs::canonicalize(&ser_path).unwrap_or_else(|_| ser_path.clone());
    println!("💾 serialize secrets");
    std::process::Command::new("sh")
        .arg(&ser_abs)
        .env("out", out)
        .env("machine", machine_name)
        .env("artifact", &artifact.name)
        .status()
        .with_context(|| format!("running serialize {}", ser_abs.display()))?;
    Ok(())
}
