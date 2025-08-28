use crate::backend::helpers::resolve_path;
use crate::config::make::ArtifactDef;
use anyhow::{Context, Result, bail};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

/// Verify that the generator produced exactly the expected files for the given artifact
pub fn verify_generated_files(artifact: &ArtifactDef, out_path: &Path) -> Result<()> {
    let expected_files: HashSet<String> = artifact.files.iter().map(|f| f.name.clone()).collect();

    let actual_files: HashSet<String> = fs::read_dir(out_path)
        .with_context(|| format!("reading generator output dir {}", out_path.display()))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().map(|t| t.is_file()).unwrap_or(false))
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .collect();

    let mut missing_files: Vec<String> =
        expected_files.difference(&actual_files).cloned().collect();

    if !missing_files.is_empty() {
        missing_files.sort();
        return Err(anyhow::anyhow!(
            "generator missing required files for artifact '{}': [{}]",
            artifact.name,
            missing_files.join(", ")
        ));
    }

    let mut unwanted_files: Vec<String> =
        actual_files.difference(&expected_files).cloned().collect();

    if !unwanted_files.is_empty() {
        unwanted_files.sort();
        return Err(anyhow::anyhow!(
            "generator produced extra files for artifact '{}': [{}]",
            artifact.name,
            unwanted_files.join(", ")
        ));
    }

    Ok(())
}

// todo: get rid of nix-shell and use bwrap directly (brwap must be part of the nix-package)
pub fn run_generator_script(
    artifact: &ArtifactDef,
    make_base: &Path,
    prompts: &Path,
    out: &Path,
) -> Result<()> {
    let generator_script = artifact.generator.as_ref();
    let generator_script_path = resolve_path(make_base, generator_script);
    let generator_script_absolut_path =
        fs::canonicalize(&generator_script_path).unwrap_or_else(|_| generator_script_path.clone());

    // Only use nix-shell. If it fails, return an error to stop the program gracefully.
    let nix_shell = which::which("nix-shell")
        .context("nix-shell is required to run the generator but was not found in PATH")?;

    // Build the bwrap command as a single string for nix-shell --run
    // Start with the always-present arguments using vec![] to appease clippy
    let mut arguments: Vec<String> = vec![
        "bwrap".to_string(),
        "--ro-bind".to_string(),
        "/nix/store".to_string(),
        "/nix/store".to_string(),
        "--tmpfs".to_string(),
        "/usr/lib/systemd".to_string(),
        "--dev".to_string(),
        "/dev".to_string(),
        "--bind".to_string(),
        prompts.display().to_string(),
        prompts.display().to_string(),
        "--bind".to_string(),
        out.display().to_string(),
        out.display().to_string(),
    ];
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
    let status = generator_command
        .status()
        .context("failed to start generator in nix-shell")?;

    if !status.success() {
        bail!(
            "generator failed inside nix-shell with exit status: {}",
            status
        );
    }

    Ok(())
}
