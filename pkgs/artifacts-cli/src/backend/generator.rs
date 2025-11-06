use crate::backend::helpers::{escape_single_quoted, fnv1a64, resolve_path};
use crate::config::make::ArtifactDef;
use crate::string_vec;
use anyhow::{Context, Result, bail};
use log::{debug, trace};
use std::collections::HashSet;
use std::fs;
use std::path::Path;

/// Verify that the generator produced exactly the expected files for the given artifact
pub fn verify_generated_files(artifact: &ArtifactDef, out_path: &Path) -> Result<()> {
    let expected_files: HashSet<String> = artifact.files.keys().cloned().collect();

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
    machine: &str,
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

    // Prepare a temporary /etc/passwd to be bind-mounted read-only inside bwrap
    // Requirement: entries "user:1000:1000:/tmp:/bin/sh"
    let passwd_content = "user:x:1000:1000::/tmp:/bin/sh\n";

    let out_path_str = out.display().to_string();
    let hash = fnv1a64(&out_path_str);
    let mut temp_passwd_path = std::env::temp_dir();
    let file_name = format!("artifacts-cli-passwd-{:016x}.txt", hash);
    temp_passwd_path.push(file_name);

    fs::write(&temp_passwd_path, passwd_content).with_context(|| {
        format!(
            "failed to create temporary passwd file at {}",
            temp_passwd_path.display()
        )
    })?;

    // Build the bwrap command as a single string for nix-shell --run
    // Start with the always-present arguments using vec![] to appease clippy
    let mut arguments: Vec<String> = string_vec!["bwrap"];
    arguments.extend(string_vec!["--unshare-all", "--unshare-user"]);
    arguments.extend(string_vec!["--uid", "1000"]);
    arguments.extend(string_vec!["--gid", "1000"]);
    arguments.extend(string_vec!["--tmpfs", "/"]);
    arguments.extend(string_vec!["--chdir", "/"]);
    arguments.extend(string_vec!["--ro-bind", "/nix/store", "/nix/store"]);
    arguments.extend(string_vec!["--tmpfs", "/usr/lib/systemd"]);
    arguments.extend(string_vec!["--proc", "/proc"]);
    arguments.extend(string_vec!["--dev", "/dev"]);
    arguments.extend(string_vec!["--bind", prompts.display(), prompts.display()]);
    arguments.extend(string_vec!["--bind", out.display(), out.display()]);
    if let Some(gen_dir) = generator_script_absolut_path.parent() {
        arguments.extend(string_vec![
            "--ro-bind",
            gen_dir.display(),
            gen_dir.display()
        ]);
    }
    if Path::new("/bin").exists() {
        arguments.extend(string_vec!["--ro-bind", "/bin", "/bin"]);
    }
    if Path::new("/usr/bin").exists() {
        arguments.extend(string_vec!["--ro-bind", "/usr/bin", "/usr/bin"]);
    }
    // Bind our custom passwd into the namespace as read-only
    arguments.extend(string_vec![
        "--ro-bind",
        temp_passwd_path.display(),
        "/etc/passwd"
    ]);
    arguments.extend(string_vec!["--", "/bin/sh"]);
    arguments.push(generator_script_absolut_path.display().to_string());
    let bwrap_command = arguments.join(" ");
    // Pretty-print the bwrap command for readability in logs
    let bwrap_pretty = {
        let mut result = String::new();
        for (index, argument) in arguments.iter().enumerate() {
            if index == 0 {
                result.push_str(argument);
            } else if argument.starts_with("--") {
                // start a new line for option keys (including the standalone "--")
                result.push_str(" \\\n");
                result.push_str(argument);
            } else {
                // keep values on the same line as their preceding key
                result.push(' ');
                result.push_str(argument);
            }
        }
        result
    };
    // Keep the original prefix but print as multiline for readability
    debug!(
        "run bwrap with command {}",
        generator_script_absolut_path.display()
    );
    trace!("{}", bwrap_pretty);

    // Ensure that our 'out' and 'prompts' override any nix-shell provided 'out'
    let out_quoted = escape_single_quoted(&out.display().to_string());
    let prompts_quoted = escape_single_quoted(&prompts.display().to_string());
    let machine_quoted = escape_single_quoted(machine);
    let artifact_quoted = escape_single_quoted(&artifact.name);
    let nix_shell_run_command = format!(
        "export out='{}'; export prompts='{}'; export machine='{}'; export artifact='{}'; {}",
        out_quoted, prompts_quoted, machine_quoted, artifact_quoted, bwrap_command
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

    // Best-effort cleanup of the temporary passwd file
    let _ = fs::remove_file(&temp_passwd_path);

    if !status.success() {
        bail!(
            "generator failed inside nix-shell with exit status: {}",
            status
        );
    }

    Ok(())
}
