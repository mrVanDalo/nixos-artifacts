//! Generator module for executing artifact generators and verifying output.
//!
//! This module handles the execution of generator scripts that produce artifact files.
//! Generators are external scripts that create files in a temporary output directory
//! based on user-provided prompts. The module ensures generators produce exactly the
//! expected files and runs them in isolated bubblewrap containers for security.
//!
//! # Generator Scripts
//!
//! Generator scripts are executed in a bubblewrap container with:
//! - `$prompts` directory containing prompt values as files
//! - `$out` directory where generated files should be written
//! - Environment variables: `$out`, `$prompts`, `$artifact_context`, `$machine`/`$username`, `$artifact`
//!
//! # Verification
//!
//! After generation, `verify_generated_files` ensures:
//! - All expected files (defined in artifact.files) are present
//! - No extra files are generated
//! - Files match the expected set exactly
//!
//! # Security
//!
//! Generator execution uses bubblewrap for containerization:
//! - Separate user namespace with uid/gid 1000
//! - Read-only bind mounts for system directories
//! - Minimal writable directories (only $out, $prompts)
//! - Custom /etc/passwd for isolation

use crate::app::model::TargetType;
use crate::backend::helpers::{escape_single_quoted, fnv1a64, resolve_path};
use crate::backend::output_capture::{CapturedOutput, run_with_captured_output};
use crate::config::make::ArtifactDef;
use crate::log_debug;
use crate::string_vec;
use anyhow::{Context, Result, bail};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;

/// Resolve and canonicalize a generator script path.
fn resolve_generator_path(make_base: &Path, generator_script: &str) -> PathBuf {
    let resolved = resolve_path(make_base, generator_script);
    fs::canonicalize(&resolved).unwrap_or(resolved)
}

/// Create a temporary passwd file for bwrap container isolation.
fn create_temp_passwd(out: &Path) -> Result<PathBuf> {
    let passwd_content = "user:x:1000:1000::/tmp:/bin/sh\n";
    let hash = fnv1a64(&out.display().to_string());
    let mut temp_path = std::env::temp_dir();
    temp_path.push(format!("artifacts-cli-passwd-{:016x}.txt", hash));
    fs::write(&temp_path, passwd_content).with_context(|| {
        format!(
            "failed to create temporary passwd file at {}",
            temp_path.display()
        )
    })?;
    Ok(temp_path)
}

/// Build bwrap command arguments for container isolation.
fn build_bwrap_arguments(
    generator_path: &Path,
    prompts: &Path,
    out: &Path,
    temp_passwd: &Path,
) -> Vec<String> {
    let mut args = string_vec!["bwrap"];
    args.extend(string_vec!["--unshare-all", "--unshare-user"]);
    args.extend(string_vec!["--uid", "1000"]);
    args.extend(string_vec!["--gid", "1000"]);
    args.extend(string_vec!["--tmpfs", "/"]);
    args.extend(string_vec!["--chdir", "/"]);
    args.extend(string_vec!["--ro-bind", "/nix/store", "/nix/store"]);
    args.extend(string_vec!["--tmpfs", "/usr/lib/systemd"]);
    args.extend(string_vec!["--proc", "/proc"]);
    args.extend(string_vec!["--dev", "/dev"]);
    args.extend(string_vec!["--bind", prompts.display(), prompts.display()]);
    args.extend(string_vec!["--bind", out.display(), out.display()]);
    if let Some(gen_dir) = generator_path.parent() {
        args.extend(string_vec![
            "--ro-bind",
            gen_dir.display(),
            gen_dir.display()
        ]);
    }
    if Path::new("/bin").exists() {
        args.extend(string_vec!["--ro-bind", "/bin", "/bin"]);
    }
    if Path::new("/usr/bin").exists() {
        args.extend(string_vec!["--ro-bind", "/usr/bin", "/usr/bin"]);
    }
    args.extend(string_vec![
        "--ro-bind",
        temp_passwd.display(),
        "/etc/passwd"
    ]);
    args.extend(string_vec!["--", "/bin/sh"]);
    args.push(generator_path.display().to_string());
    args
}

/// Build environment exports for artifact-specific generators.
fn build_env_exports(
    out: &Path,
    prompts: &Path,
    target_type: &TargetType,
    artifact_name: &str,
) -> String {
    let out_quoted = escape_single_quoted(&out.display().to_string());
    let prompts_quoted = escape_single_quoted(&prompts.display().to_string());
    let artifact_quoted = escape_single_quoted(artifact_name);
    let context_quoted = escape_single_quoted(target_type.context_str());

    match target_type {
        TargetType::HomeManager { username } => {
            let username_quoted = escape_single_quoted(username);
            format!(
                "export out='{}'; export prompts='{}'; export artifact_context='{}'; export username='{}'; export artifact='{}';",
                out_quoted, prompts_quoted, context_quoted, username_quoted, artifact_quoted
            )
        }
        TargetType::NixOS { machine } => {
            let machine_quoted = escape_single_quoted(machine);
            format!(
                "export out='{}'; export prompts='{}'; export artifact_context='{}'; export machine='{}'; export artifact='{}';",
                out_quoted, prompts_quoted, context_quoted, machine_quoted, artifact_quoted
            )
        }
        TargetType::Shared { .. } => {
            format!(
                "export out='{}'; export prompts='{}'; export artifact_context='{}'; export artifact='{}';",
                out_quoted, prompts_quoted, context_quoted, artifact_quoted
            )
        }
    }
}

/// Build environment exports for shared artifact generators.
fn build_shared_env_exports(out: &Path, prompts: &Path) -> String {
    let out_quoted = escape_single_quoted(&out.display().to_string());
    let prompts_quoted = escape_single_quoted(&prompts.display().to_string());
    format!(
        "export out='{}'; export prompts='{}'; export artifact_context='shared';",
        out_quoted, prompts_quoted
    )
}

/// Execute generator in nix-shell with bwrap containerization.
fn execute_generator_in_bwrap(
    nix_shell: &Path,
    arguments: &[String],
    env_exports: &str,
) -> Result<CapturedOutput> {
    let bwrap_command = arguments.join(" ");
    let run_command = format!("{} {}", env_exports, bwrap_command);

    let mut cmd = std::process::Command::new(nix_shell);
    cmd.arg("-p")
        .arg("bash")
        .arg("bubblewrap")
        .arg("--run")
        .arg(&run_command)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let child = cmd
        .spawn()
        .context("failed to start generator in nix-shell")?;
    run_with_captured_output(child).context("failed to capture generator output")
}

/// Log the bwrap command with formatted output.
#[cfg(feature = "logging")]
fn log_bwrap_command(arguments: &[String]) {
    let mut result = String::new();
    for (index, argument) in arguments.iter().enumerate() {
        if index == 0 {
            result.push_str(argument);
        } else if argument.starts_with("--") {
            result.push_str(" \\\n");
            result.push_str(argument);
        } else {
            result.push(' ');
            result.push_str(argument);
        }
    }
    log_debug!("{}", result);
}

/// Log the bwrap command (no-op when logging is disabled).
#[cfg(not(feature = "logging"))]
fn log_bwrap_command(_arguments: &[String]) {}

/// Verify that the generator produced exactly the expected files for the given artifact.
///
/// This function checks that the generator script produced:
/// - All files defined in `artifact.files` (no missing files)
/// - No additional files not defined in `artifact.files` (no extra files)
///
/// # Arguments
///
/// * `artifact` - The artifact definition containing the expected file set
/// * `out_path` - Path to the directory where generator output is located
///
/// # Returns
///
/// Returns `Ok(())` if the file set matches exactly.
///
/// # Errors
///
/// Returns an error if:
/// - The output directory cannot be read
/// - Required files are missing from the output
/// - Extra files were generated that aren't in the artifact definition
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

/// Run a generator script for an artifact in an isolated bubblewrap container.
///
/// This function executes the artifact's generator script inside a nix-shell environment
/// with bubblewrap containerization for security. The generator script is expected to
/// create files in the `$out` directory based on values from the `$prompts` directory.
///
/// # Environment Variables
///
/// The generator script receives these environment variables:
/// * `$out` - Path to the output directory where files should be created
/// * `$prompts` - Path to directory containing prompt values as files
/// * `$artifact_context` - Context string: "nixos", "homemanager", or "shared"
/// * `$machine` - Machine name (for nixos context) or `$username` (for homemanager context)
/// * `$artifact` - Name of the artifact being generated
///
/// # Arguments
///
/// * `artifact` - The artifact definition containing the generator script path
/// * `target_type` - Target type with name (Nixos { machine }, HomeManager { username }, or Shared)
/// * `make_base` - Base path for resolving relative script paths
/// * `prompts` - Directory containing prompt values as files
/// * `out` - Directory where generator should create output files
///
/// # Returns
///
/// Returns the captured output from the generator script execution.
///
/// # Errors
///
/// Returns an error if:
/// - nix-shell is not found in PATH
/// - The generator script cannot be found or executed
/// - The generator script exits with non-zero status
pub fn run_generator_script(
    artifact: &ArtifactDef,
    target_type: &TargetType,
    make_base: &Path,
    prompts: &Path,
    out: &Path,
) -> Result<CapturedOutput> {
    let generator_path = resolve_generator_path(make_base, artifact.generator.as_ref());
    let nix_shell = which::which("nix-shell")
        .context("nix-shell is required to run the generator but was not found in PATH")?;
    let temp_passwd = create_temp_passwd(out)?;
    let arguments = build_bwrap_arguments(&generator_path, prompts, out, &temp_passwd);

    log_debug!("run bwrap with command {}", generator_path.display());
    log_bwrap_command(&arguments);

    let env_exports = build_env_exports(out, prompts, target_type, &artifact.name);
    let output = execute_generator_in_bwrap(&nix_shell, &arguments, &env_exports)?;

    let _ = fs::remove_file(&temp_passwd);

    if !output.exit_success {
        bail!("generator failed inside nix-shell with non-zero exit status");
    }

    Ok(output)
}

/// Run a generator script by its direct path (for shared artifacts).
///
/// This is a simplified version of `run_generator_script` that doesn't require
/// a full artifact definition. It's used for shared artifacts where the generator
/// path is known directly rather than extracted from an artifact definition.
///
/// The generator receives a minimal environment:
/// * `$out` - Path to the output directory where files should be created
/// * `$prompts` - Path to directory containing prompt values as files
/// * `$artifact_context` - Always set to "shared" for this function
///
/// # Arguments
///
/// * `generator_path` - Path to the generator script to execute
/// * `make_base` - Base path for resolving relative script paths
/// * `prompts` - Directory containing prompt values as files
/// * `out` - Directory where generator should create output files
///
/// # Returns
///
/// Returns the captured output from the generator script execution.
///
/// # Errors
///
/// Returns an error if:
/// - nix-shell is not found in PATH
/// - The generator script cannot be found or executed
/// - The generator script exits with non-zero status
///
/// # Example
///
/// ```rust,ignore
/// let output = run_generator_script_with_path(
///     "./generators/ssh_key.sh",
///     Path::new("/project"),
///     Path::new("/tmp/prompts"),
///     Path::new("/tmp/out"),
/// )?;
/// assert!(output.exit_success);
/// ```
pub fn run_generator_script_with_path(
    generator_path: &str,
    make_base: &Path,
    prompts: &Path,
    out: &Path,
) -> Result<CapturedOutput> {
    let resolved_generator_path = resolve_generator_path(make_base, generator_path);
    let nix_shell = which::which("nix-shell")
        .context("nix-shell is required to run the generator but was not found in PATH")?;
    let temp_passwd = create_temp_passwd(out)?;
    let arguments = build_bwrap_arguments(&resolved_generator_path, prompts, out, &temp_passwd);

    log_debug!(
        "run shared generator bwrap with command {}",
        resolved_generator_path.display()
    );
    log_bwrap_command(&arguments);

    let env_exports = build_shared_env_exports(out, prompts);
    let output = execute_generator_in_bwrap(&nix_shell, &arguments, &env_exports)?;

    let _ = fs::remove_file(&temp_passwd);

    if !output.exit_success {
        bail!("shared generator failed inside nix-shell with non-zero exit status");
    }

    Ok(output)
}
