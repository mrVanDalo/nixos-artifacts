use super::prompt::PromptManager;
use crate::config::backend::BackendConfig;
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
    // In tests we want deterministic paths to produce stable snapshots.
    let base = if std::env::var("ARTIFACTS_TUI_TEST_FIXED_TMP").is_ok() {
        let base = std::env::temp_dir().join("artifacts-tui-test");
        // Clean up any leftovers from previous runs
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).context("creating test base directory")?;
        base
    } else {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis();
        std::env::temp_dir().join(format!("artifacts-tui-{}", now))
    };
    let prompts = base.join("prompts");
    let out = base.join("out");
    fs::create_dir_all(&prompts).context("creating prompts directory")?;
    fs::create_dir_all(&out).context("creating out directory")?;
    Ok((base, prompts, out))
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
    backend_cfg: &BackendConfig,
    backend_base: &Path,
    make_base: &Path,
    backend_toml: &Path,
) -> bool {
    let Some(backend_name) = artifact.serialization.as_ref() else {
        println!("    no serialization backend defined");
        return false;
    };

    let Some(entry) = backend_cfg.get(backend_name) else {
        println!(
            "    WARN: backend '{}' not found in {}",
            backend_name,
            backend_toml.display()
        );
        return false;
    };

    // Create per-artifact inputs dir and populate with JSON files
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let safe_art = artifact
        .name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect::<String>();
    let inputs = if std::env::var("ARTIFACTS_TUI_TEST_FIXED_TMP").is_ok() {
        std::env::temp_dir().join(format!("artifacts-tui-test-{}-inputs", safe_art))
    } else {
        std::env::temp_dir().join(format!("artifacts-tui-{}-{}-inputs", now, safe_art))
    };

    let mut skip_rest = false;

    if let Err(e) = fs::create_dir_all(&inputs) {
        println!(
            "    ERROR: failed to create inputs dir {}: {}",
            inputs.display(),
            e
        );
        return false;
    }

    for f in &artifact.files {
        let resolved_path = resolve_path(make_base, &f.path);
        let file_name = f
            .name
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect::<String>();
        let json_path = inputs.join(format!("{}.json", file_name));
        let obj = serde_json::json!({
            "path": resolved_path,
            "owner": f.owner,
            "group": f.group,
        });
        match serde_json::to_string_pretty(&obj) {
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
        inputs.display(),
        machine,
        artifact.name,
        check_abs.display()
    );
    let status = std::process::Command::new(&check_abs)
        .env("inputs", &inputs)
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

    if let Err(e) = fs::remove_dir_all(&inputs) {
        println!(
            "    WARN: failed to remove inputs dir {}: {}",
            inputs.display(),
            e
        );
    }

    skip_rest
}

fn run_generator(artifact: &ArtifactDef, make_base: &Path, prompts: &Path, out: &Path) {
    if let Some(generator_script) = artifact.generator.as_ref() {
        let generator_script_path = resolve_path(make_base, generator_script);
        let generator_script_absolut_path = fs::canonicalize(&generator_script_path)
            .unwrap_or_else(|_| generator_script_path.clone());

        // Only use nix-shell. If it fails, crash the program.
        let nix_shell = which::which("nix-shell")
            .expect("nix-shell is required to run the generator but was not found in PATH");

        // Build the bwrap command as a single string for nix-shell --run
        let mut arguments: Vec<String> = Vec::new();
        arguments.push("bwrap".to_string());
        arguments.push("--ro-bind".to_string());
        arguments.push("/nix/store".to_string());
        arguments.push("/nix/store".to_string());
        arguments.push("--tmpfs".to_string());
        arguments.push("/usr/lib/systemd".to_string());
        arguments.push("--dev".to_string());
        arguments.push("/dev".to_string());
        arguments.push("--bind".to_string());
        arguments.push(prompts.display().to_string());
        arguments.push(prompts.display().to_string());
        arguments.push("--bind".to_string());
        arguments.push(out.display().to_string());
        arguments.push(out.display().to_string());
        if let Some(gen_dir) = generator_script_absolut_path.parent() {
            arguments.push("--ro-bind".to_string());
            arguments.push(gen_dir.display().to_string());
            arguments.push(gen_dir.display().to_string());
        }
        if Path::new("/bin").exists() {
            arguments.push("--ro-bind".to_string());
            arguments.push("/bin".to_string());
            arguments.push("/bin".to_string());
        }
        if Path::new("/usr/bin").exists() {
            arguments.push("--ro-bind".to_string());
            arguments.push("/usr/bin".to_string());
            arguments.push("/usr/bin".to_string());
        }
        arguments.push("--unshare-all".to_string());
        arguments.push("--unshare-user".to_string());
        arguments.push("--uid".to_string());
        arguments.push("1000".to_string());
        arguments.push("--".to_string());
        arguments.push("/bin/sh".to_string());
        arguments.push(generator_script_absolut_path.display().to_string());
        let bwrap_command = arguments.join(" ");

        // Ensure that our 'out' and 'prompt' override any nix-shell provided 'out'
        fn sh_escape_single_quoted(s: &str) -> String {
            // Replace ' with '\'' for safe single-quoting
            s.replace('\'', "'\\''")
        }
        let out_quoted = sh_escape_single_quoted(&out.display().to_string());
        let prompt_quoted = sh_escape_single_quoted(&prompts.display().to_string());
        let nix_shell_run_command = format!(
            "export out='{}'; export prompt='{}'; {}",
            out_quoted, prompt_quoted, bwrap_command
        );

        let mut generator_command = std::process::Command::new(nix_shell);
        generator_command
            .arg("-p")
            .arg("bash")
            .arg("bubblewrap")
            .arg("--run")
            .arg(&nix_shell_run_command);

        // Do not pass 'out' or 'prompt' here to avoid being overridden by nix-shell internals
        match generator_command.status() {
            Ok(status) => {
                if !status.success() {
                    panic!(
                        "generator failed inside nix-shell with exit status: {}",
                        status
                    );
                }
            }
            Err(e) => {
                panic!("failed to start generator in nix-shell: {}", e);
            }
        }
    } else {
        panic!("no generator defined");
    }
}

fn run_serialize(
    artifact: &ArtifactDef,
    backend_cfg: &BackendConfig,
    backend_base: &Path,
    out: &Path,
    machine: &str,
    backend_toml: &Path,
) {
    if let Some(backend_name) = artifact.serialization.as_ref() {
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
    } else {
        println!("    no serialization backend defined");
    }
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
    let prompt_manager = PromptManager::new();

    for (machine, artifacts) in make_map.drain() {
        println!("[generate] machine: {}", machine);
        for artifact in artifacts {
            println!("  - artifact: {}", artifact.name);
            // Do not print prompts to stdout; inputs are read from stdin and stored in $prompt
            print_files(&artifact, make_base);

            // First, check if we can skip generation/serialization
            let skip_rest = maybe_run_check_serialization(
                &artifact,
                &machine,
                backend_cfg,
                backend_base,
                make_base,
                backend_toml,
            );
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

            if let Err(e) = prompt_results.write_prompts_to_files(prompts) {
                eprintln!("Error writing prompt files: {}", e);
            }

            run_generator(&artifact, make_base, prompts, out);

            run_serialize(
                &artifact,
                backend_cfg,
                backend_base,
                out,
                &machine,
                backend_toml,
            );
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
