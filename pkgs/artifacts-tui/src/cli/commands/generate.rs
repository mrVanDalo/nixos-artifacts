use crate::config::backend::{BackendConfig, BackendEntry};
use crate::config::make::ArtifactDef;
use anyhow::{Context, Result, bail};
use serde_json::from_str as json_from_str;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

fn resolve_path(base_dir: &Path, relative_path: &str) -> PathBuf {
    let path = Path::new(relative_path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base_dir.join(path)
    }
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

fn prepare_temp_dirs() -> Result<(PathBuf, PathBuf, PathBuf)> {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let base = std::env::temp_dir().join(format!("artifacts-tui-{}", now));
    let prompts = base.join("prompts");
    let out = base.join("out");
    fs::create_dir_all(&prompts).context("creating prompts directory")?;
    fs::create_dir_all(&out).context("creating out directory")?;
    Ok((base, prompts, out))
}

fn process_plan(
    mut make_map: HashMap<String, Vec<ArtifactDef>>,
    make_base: &Path,
    backend_cfg: &BackendConfig,
    backend_base: &Path,
    prompts: &Path,
    out: &Path,
    backend_toml: &Path,
) {
    for (machine, artifacts) in make_map.drain() {
        println!("[generate] machine: {}", machine);
        for artifact in artifacts {
            println!("  - artifact: {}", artifact.name);
            if !artifact.prompts.is_empty() {
                println!(
                    "    prompts to collect -> {} entries",
                    artifact.prompts.len()
                );
                for p in &artifact.prompts {
                    println!(
                        "      - {}{}",
                        p.name,
                        p.description
                            .as_ref()
                            .map(|d| format!(" ({})", d))
                            .unwrap_or_default()
                    );
                }
            }
            if !artifact.files.is_empty() {
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

            if let Some(generator_script) = artifact.generator.as_ref() {
                let gen_path = resolve_path(make_base, generator_script);
                let gen_abs = fs::canonicalize(&gen_path).unwrap_or_else(|_| gen_path.clone());
                println!(
                    "    would run generator: env prompt=\"{}\" out=\"{}\" {}\n",
                    prompts.display(),
                    out.display(),
                    gen_abs.display()
                );
                println!("    generator script path: {}", gen_abs.display());
                match fs::read_to_string(&gen_abs) {
                    Ok(content) => println!("    generator script content:\n{}", content),
                    Err(e) => println!("    failed to read generator script: {}", e),
                }
            } else {
                println!("    no generator defined");
            }

            if let Some(backend_name) = artifact.serialization.as_ref() {
                match backend_cfg.get(backend_name) {
                    Some(BackendEntry { serialize, .. }) => {
                        let ser_path = resolve_path(backend_base, serialize);
                        let ser_abs =
                            fs::canonicalize(&ser_path).unwrap_or_else(|_| ser_path.clone());
                        println!(
                            "    would run serialize: env out=\"{}\" {}\n",
                            out.display(),
                            ser_abs.display()
                        );
                        println!("    serialize script path: {}", ser_abs.display());
                        match fs::read_to_string(&ser_abs) {
                            Ok(content) => println!("    serialize script content:\n{}", content),
                            Err(e) => println!("    failed to read serialize script: {}", e),
                        }
                    }
                    None => {
                        println!(
                            "    WARN: backend '{}' not found in {}",
                            backend_name,
                            backend_toml.display()
                        );
                    }
                }
            } else {
                println!("    no serialization backend defined");
            }
        }
    }
}

/// Generate plan: read make.json and backend config and print scripts to run.
pub fn run(backend_toml: &Path, make_json: &Path) -> Result<()> {
    // Load backend config (TOML), paths relative to file location
    let (backend_cfg, backend_base) = read_backend_config(backend_toml)?;

    // Load make.json. The format is: { "<machine-name>": [ArtifactDef, ...], ... }
    let (make_map, make_base) = read_make_config(make_json)?;

    // Prepare temp dirs used by scripts; we still only print the plan and do not execute.
    let (base, prompts, out) = prepare_temp_dirs()?;

    println!("[generate] backend: {}", backend_toml.display());
    println!("[generate] make: {}", make_json.display());
    println!("[generate] prompts dir: {}", prompts.display());
    println!("[generate] out dir: {}", out.display());

    // Iterate machines and artifacts
    process_plan(
        make_map,
        &make_base,
        &backend_cfg,
        &backend_base,
        &prompts,
        &out,
        backend_toml,
    );

    // For now, just verify directories exist and then clean up
    if !prompts.is_dir() || !out.is_dir() {
        bail!("failed to prepare temporary directories");
    }
    fs::remove_dir_all(&base).with_context(|| format!("removing temp base {}", base.display()))?;

    Ok(())
}
