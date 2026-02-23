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

use crate::backend::helpers::{escape_single_quoted, fnv1a64, resolve_path};
use crate::backend::output_capture::{run_with_captured_output, CapturedOutput};
use crate::config::make::ArtifactDef;
use crate::log_debug;
#[cfg(feature = "logging")]
use crate::log_trace;
use crate::string_vec;
use anyhow::{bail, Context, Result};
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::process::Stdio;

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
/// * `machine` - Target machine or username for this artifact
/// * `make_base` - Base path for resolving relative script paths
/// * `prompts` - Directory containing prompt values as files
/// * `out` - Directory where generator should create output files
/// * `context` - Context string: "nixos", "homemanager", or "shared"
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
    machine: &str,
    make_base: &Path,
    prompts: &Path,
    out: &Path,
    context: &str,
) -> Result<CapturedOutput> {
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
    // Pretty-print the bwrap command for readability in logs (only when logging feature is enabled)
    #[cfg(feature = "logging")]
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
    log_debug!(
        "run bwrap with command {}",
        generator_script_absolut_path.display()
    );
    #[cfg(feature = "logging")]
    log_trace!("{}", bwrap_pretty);

    // Ensure that our 'out' and 'prompts' override any nix-shell provided 'out'
    let out_quoted = escape_single_quoted(&out.display().to_string());
    let prompts_quoted = escape_single_quoted(&prompts.display().to_string());
    let machine_quoted = escape_single_quoted(machine);
    let artifact_quoted = escape_single_quoted(&artifact.name);
    let context_quoted = escape_single_quoted(context);

    // Build env exports depending on context
    let env_exports = if context == "homemanager" {
        format!(
            "export out='{}'; export prompts='{}'; export artifact_context='{}'; export username='{}'; export artifact='{}';",
            out_quoted, prompts_quoted, context_quoted, machine_quoted, artifact_quoted
        )
    } else {
        format!(
            "export out='{}'; export prompts='{}'; export artifact_context='{}'; export machine='{}'; export artifact='{}';",
            out_quoted, prompts_quoted, context_quoted, machine_quoted, artifact_quoted
        )
    };

    let nix_shell_run_command = format!("{} {}", env_exports, bwrap_command);

    let mut generator_command = std::process::Command::new(nix_shell);
    generator_command
        .arg("-p")
        .arg("bash")
        .arg("bubblewrap")
        .arg("--run")
        .arg(&nix_shell_run_command)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // Do not pass 'out' or 'prompt' here to avoid being overridden by nix-shell internals
    let child = generator_command
        .spawn()
        .context("failed to start generator in nix-shell")?;

    let output = run_with_captured_output(child).context("failed to capture generator output")?;

    // Best-effort cleanup of the temporary passwd file
    let _ = fs::remove_file(&temp_passwd_path);

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
    let generator_script_path = resolve_path(make_base, generator_path);
    let generator_script_absolut_path =
        fs::canonicalize(&generator_script_path).unwrap_or_else(|_| generator_script_path.clone());

    let nix_shell = which::which("nix-shell")
        .context("nix-shell is required to run the generator but was not found in PATH")?;

    // Prepare a temporary /etc/passwd to be bind-mounted read-only inside bwrap
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

    // Build the bwrap command
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
    arguments.extend(string_vec![
        "--ro-bind",
        temp_passwd_path.display(),
        "/etc/passwd"
    ]);
    arguments.extend(string_vec!["--", "/bin/sh"]);
    arguments.push(generator_script_absolut_path.display().to_string());
    let bwrap_command = arguments.join(" ");

    log_debug!(
        "run shared generator bwrap with command {}",
        generator_script_absolut_path.display()
    );

    // Build env exports for shared artifact (no specific target)
    let out_quoted = escape_single_quoted(&out.display().to_string());
    let prompts_quoted = escape_single_quoted(&prompts.display().to_string());

    let env_exports = format!(
        "export out='{}'; export prompts='{}'; export artifact_context='shared';",
        out_quoted, prompts_quoted
    );

    let nix_shell_run_command = format!("{} {}", env_exports, bwrap_command);

    let mut generator_command = std::process::Command::new(nix_shell);
    generator_command
        .arg("-p")
        .arg("bash")
        .arg("bubblewrap")
        .arg("--run")
        .arg(&nix_shell_run_command)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let child = generator_command
        .spawn()
        .context("failed to start generator in nix-shell")?;

    let output = run_with_captured_output(child).context("failed to capture generator output")?;

    let _ = fs::remove_file(&temp_passwd_path);

    if !output.exit_success {
        bail!("shared generator failed inside nix-shell with non-zero exit status");
    }

    Ok(output)
}
