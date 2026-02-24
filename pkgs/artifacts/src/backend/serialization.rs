//! Serialization module for artifact storage and retrieval operations.
//!
//! This module handles the execution of backend scripts for checking serialization status,
//! serializing generated artifacts, and deserializing stored artifacts. All operations
//! support both single-target (NixOS/HomeManager) and shared artifact scenarios.
//!
//! # Backend Operations
//!
//! Three main operations are supported:
//!
//! ## Check Serialization
//! Determines if an artifact needs regeneration by running the `check_serialization` script.
//! Exit code 0 means "up-to-date", any non-zero exit means "needs generation".
//!
//! ## Serialize
//! Stores generated files using the `serialize` script. The script receives:
//! - `$out` - Directory containing generated files
//! - `$config` - Path to JSON file with backend-specific configuration
//! - `$artifact` - Artifact name
//! - `$machine` or `$username` - Target identifier (context-dependent)
//!
//! ## Shared Operations
//! For shared artifacts that span multiple machines/users:
//! - `run_shared_serialize` - Serializes with machines.json and users.json context
//! - `run_shared_check_serialization` - Checks status with full context
//!
//! # Timeout Protection
//!
//! All script executions have a 30-second timeout to prevent hanging operations.
//! Timeout errors are returned as `ScriptError::Timeout`.

use crate::backend::helpers::{resolve_path, validate_backend_script};
use crate::backend::output_capture::{
    run_with_captured_output_and_timeout, CapturedOutput, ScriptError,
};
use crate::backend::tempfile::TempFile;
use crate::config::backend::{BackendConfiguration, BackendEntry};
use crate::config::make::{ArtifactDef, MakeConfiguration};
use anyhow::{bail, Context, Result};
use serde_json::{json, to_string_pretty, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

/// Timeout duration for serialization scripts (30 seconds).
///
/// This timeout applies to all backend script executions to prevent
/// indefinite hanging on misconfigured scripts or slow backends.
const SERIALIZATION_TIMEOUT: Duration = Duration::from_secs(30);

/// Result of running check_serialization script.
///
/// This struct captures both the success status and any output from
/// the check script. The `needs_generation` field indicates whether
/// the artifact should be regenerated.
pub struct CheckResult {
    /// True if the artifact needs generation, false if up-to-date.
    ///
    /// When `true`, the artifact should be regenerated before serialization.
    /// When `false`, the existing serialized state is current.
    pub needs_generation: bool,
    /// Captured stdout/stderr from the script execution.
    ///
    /// Contains the complete output from the check script, which can
    /// be useful for debugging or displaying status messages.
    pub output: CapturedOutput,
}

/// Script information extracted from backend entry based on context
struct ScriptInfo<'a> {
    script_path: Option<&'a String>,
    script_name: &'a str,
}

/// Handle the output from a script execution, converting ScriptErrors to CheckResults
#[allow(unused_variables)]
fn handle_check_output(
    result: Result<CapturedOutput, ScriptError>,
    artifact_name: &str,
) -> Result<CheckResult> {
    match result {
        Ok(out) => {
            let needs_generation = !out.exit_success;
            if out.exit_success {
                crate::log_debug!("OK -> skipping generation");
            } else {
                crate::log_debug!("needs generation");
            }
            Ok(CheckResult {
                needs_generation,
                output: out,
            })
        }
        Err(ScriptError::Timeout {
            script_name: name,
            timeout_secs,
        }) => {
            crate::log_debug!(
                "{} timed out after {}s for {}",
                name,
                timeout_secs,
                artifact_name
            );
            Ok(make_timeout_result(&name, timeout_secs))
        }
        Err(ScriptError::Io { message }) => Ok(make_io_result(&message)),
        Err(ScriptError::Failed { stderr, .. }) => Ok(make_failed_result(stderr)),
    }
}

/// Get target label for logging
#[allow(dead_code)]
fn get_target_label(context: &str) -> &str {
    if context == "homemanager" {
        "username"
    } else {
        "machine"
    }
}

/// Build machines JSON file mapping machine names to backend configs
fn build_machines_json(
    make: &MakeConfiguration,
    nixos_targets: &[String],
    backend_name: &str,
) -> Result<(TempFile, PathBuf)> {
    let dir = TempFile::new_dir("machines")?;
    let path = dir.join("machines.json");
    let config: Map<String, Value> = nixos_targets
        .iter()
        .map(|machine| {
            let config = make
                .get_backend_config_for(machine, backend_name)
                .map(|m| serde_json::to_value(m).unwrap_or(json!({})))
                .unwrap_or(json!({}));
            (machine.clone(), config)
        })
        .collect();
    let text = to_string_pretty(&config)?;
    fs::write(&path, &text).with_context(|| format!("writing {}", path.display()))?;
    Ok((dir, path))
}

/// Build users JSON file mapping user names to backend configs
fn build_users_json(
    make: &MakeConfiguration,
    home_targets: &[String],
    backend_name: &str,
) -> Result<(TempFile, PathBuf)> {
    let dir = TempFile::new_dir("users")?;
    let path = dir.join("users.json");
    let config: Map<String, Value> = home_targets
        .iter()
        .map(|user| {
            let config = make
                .get_backend_config_for(user, backend_name)
                .map(|m| serde_json::to_value(m).unwrap_or(json!({})))
                .unwrap_or(json!({}));
            (user.clone(), config)
        })
        .collect();
    let text = to_string_pretty(&config)?;
    fs::write(&path, &text).with_context(|| format!("writing {}", path.display()))?;
    Ok((dir, path))
}

/// Build config JSON file for single target serialization
fn build_config_json(
    make: &MakeConfiguration,
    target_name: &str,
    backend_name: &str,
    artifact_name: &str,
) -> Result<(TempFile, PathBuf)> {
    let dir = TempFile::new_dir_with_name(&format!("config-{}", artifact_name))?;
    let path = dir.join("config.json");
    let config = make
        .get_backend_config_for(target_name, backend_name)
        .map(|m| serde_json::to_value(m).unwrap_or(json!({})))
        .unwrap_or(json!({}));
    let text = to_string_pretty(&config)?;
    fs::write(&path, &text).with_context(|| format!("writing {}", path.display()))?;
    Ok((dir, path))
}

/// Get serialize script info based on context
fn get_serialize_script<'a>(entry: &'a BackendEntry, context: &str) -> ScriptInfo<'a> {
    if context == "homemanager" {
        ScriptInfo {
            script_path: entry.home_serialize.as_ref(),
            script_name: "home_serialize",
        }
    } else {
        ScriptInfo {
            script_path: entry.nixos_serialize.as_ref(),
            script_name: "nixos_serialize",
        }
    }
}

/// Get check_serialization script info based on context
fn get_check_script<'a>(entry: &'a BackendEntry, context: &str) -> ScriptInfo<'a> {
    if context == "homemanager" {
        ScriptInfo {
            script_path: entry.home_check_serialization.as_ref(),
            script_name: "home_check_serialization",
        }
    } else {
        ScriptInfo {
            script_path: entry.nixos_check_serialization.as_ref(),
            script_name: "nixos_check_serialization",
        }
    }
}

/// Build a Command for serialize script execution
fn build_serialize_command(
    script_path: &Path,
    out_dir: &Path,
    config_path: &Path,
    context: &str,
    target_name: &str,
    artifact_name: &str,
) -> Command {
    let mut cmd = Command::new("sh");
    cmd.arg(script_path)
        .env("out", out_dir)
        .env("config", config_path)
        .env("artifact_context", context)
        .env("artifact", artifact_name)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if context == "homemanager" {
        cmd.env("username", target_name);
    } else {
        cmd.env("machine", target_name);
    }

    #[cfg(test)]
    if let Ok(test_output_dir) = std::env::var("ARTIFACTS_TEST_OUTPUT_DIR") {
        cmd.env("ARTIFACTS_TEST_OUTPUT_DIR", test_output_dir);
    }

    cmd
}

/// Build a Command for check_serialization script execution
fn build_check_command(
    script_path: &Path,
    inputs_dir: &Path,
    config_path: &Path,
    context: &str,
    target: &str,
    artifact_name: &str,
) -> Command {
    let mut cmd = Command::new(script_path);
    cmd.env("inputs", inputs_dir)
        .env("config", config_path)
        .env("artifact_context", context)
        .env("artifact", artifact_name)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if context == "homemanager" {
        cmd.env("username", target);
    } else {
        cmd.env("machine", target);
    }

    cmd
}

/// Build a Command for shared_serialize script execution
fn build_shared_serialize_command(
    script_path: &Path,
    out_dir: &Path,
    machines_path: &Path,
    users_path: &Path,
    artifact_name: &str,
) -> Command {
    let mut cmd = Command::new("sh");
    cmd.arg(script_path)
        .env("artifact", artifact_name)
        .env("out", out_dir)
        .env("machines", machines_path)
        .env("users", users_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(test)]
    if let Ok(test_output_dir) = std::env::var("ARTIFACTS_TEST_OUTPUT_DIR") {
        cmd.env("ARTIFACTS_TEST_OUTPUT_DIR", test_output_dir);
    }

    cmd
}

/// Build a Command for shared_check_serialization script execution
fn build_shared_check_command(
    script_path: &Path,
    machines_path: &Path,
    users_path: &Path,
    artifact_name: &str,
) -> Command {
    let mut cmd = Command::new(script_path);
    cmd.env("artifact", artifact_name)
        .env("machines", machines_path)
        .env("users", users_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    cmd
}

/// Run command with timeout handling, returning output or an error
fn run_command_with_timeout(
    child: Child,
    script_name: &str,
    timeout: Duration,
) -> Result<CapturedOutput> {
    match run_with_captured_output_and_timeout(child, script_name, timeout) {
        Ok(output) => Ok(output),
        Err(ScriptError::Timeout {
            script_name: name,
            timeout_secs,
        }) => {
            bail!("{} timed out after {} seconds", name, timeout_secs);
        }
        Err(ScriptError::Io { message }) => {
            bail!("I/O error during {}: {}", script_name, message);
        }
        Err(ScriptError::Failed { exit_code, stderr }) => {
            bail!(
                "{} failed with exit code {}: {}",
                script_name,
                exit_code,
                stderr
            );
        }
    }
}

/// Create CheckResult for timeout error
fn make_timeout_result(script_name: &str, timeout_secs: u64) -> CheckResult {
    let mut output = CapturedOutput::default();
    output
        .lines
        .push(crate::backend::output_capture::OutputLine {
            stream: crate::backend::output_capture::OutputStream::Stderr,
            content: format!("{} timed out after {} seconds", script_name, timeout_secs),
        });
    CheckResult {
        needs_generation: true,
        output,
    }
}

/// Create CheckResult for I/O error
fn make_io_result(message: &str) -> CheckResult {
    let mut output = CapturedOutput::default();
    output
        .lines
        .push(crate::backend::output_capture::OutputLine {
            stream: crate::backend::output_capture::OutputStream::Stderr,
            content: format!("I/O error: {}", message),
        });
    CheckResult {
        needs_generation: true,
        output,
    }
}

/// Create CheckResult for failed script execution
fn make_failed_result(stderr: String) -> CheckResult {
    let mut output = CapturedOutput::default();
    output
        .lines
        .push(crate::backend::output_capture::OutputLine {
            stream: crate::backend::output_capture::OutputStream::Stderr,
            content: stderr,
        });
    CheckResult {
        needs_generation: true,
        output,
    }
}

/// Verify output succeeded, bail on failure.
/// Helper function for ergonomic Result propagation in scripts.
/// Kept for future use - Phase 22 will refactor serialization to use this pattern.
#[allow(dead_code)]
fn verify_output_succeeded(output: &CapturedOutput, script_name: &str) -> Result<()> {
    if !output.exit_success {
        bail!("{} script failed with non-zero exit status", script_name);
    }
    Ok(())
}

/// Write input files for check_serialization
fn write_check_input_files(
    artifact: &ArtifactDef,
    inputs_dir: &Path,
    make: &MakeConfiguration,
) -> Result<()> {
    for file in artifact.files.values() {
        let resolved_path = file
            .path
            .as_ref()
            .map(|path| resolve_path(&make.make_base, path));
        let json_path = inputs_dir.join(&file.name);

        let text = to_string_pretty(&json!({
            "path": resolved_path,
            "owner": file.owner,
            "group": file.group,
        }))?;

        fs::write(&json_path, text).with_context(|| format!("writing {}", json_path.display()))?;
    }
    Ok(())
}

/// Run the serialize script for a generated artifact.
///
/// Executes the serialize script from the backend configuration to store
/// the generated artifact files. The script receives paths to the output
/// directory and a configuration JSON file.
///
/// # Arguments
///
/// * `artifact` - The artifact definition containing serialization backend
/// * `backend` - The backend configuration with script paths
/// * `out` - Directory containing the generated files to serialize
/// * `target_name` - Name of the target (machine or username)
/// * `make` - The make configuration for backend settings
/// * `context` - Context string: "nixos", "homemanager", or "shared"
///
/// # Returns
///
/// Returns the captured output from the serialize script.
///
/// # Errors
///
/// Returns an error if:
/// - The backend doesn't have a serialize script configured
/// - The serialize script cannot be found or executed
/// - The script times out (after 30 seconds)
/// - The script exits with non-zero status
///
/// # Context-Specific Behavior
///
/// For "nixos" context: Sets `$machine` environment variable
/// For "homemanager" context: Sets `$username` environment variable
pub fn run_serialize(
    artifact: &ArtifactDef,
    backend: &BackendConfiguration,
    out: &Path,
    target_name: &str,
    make: &MakeConfiguration,
    context: &str,
) -> Result<CapturedOutput> {
    let backend_name = &artifact.serialization;
    let entry = backend.get_backend(backend_name)?;

    let script_info = get_serialize_script(&entry, context);
    let script_path = script_info.script_path.ok_or_else(|| {
        anyhow::anyhow!(
            "backend '{}' has no '{}' script configured",
            backend_name,
            script_info.script_name
        )
    })?;

    let script_abs = validate_backend_script(
        backend_name,
        script_info.script_name,
        &backend.base_path,
        script_path,
    )?;

    let (_, config_file) = build_config_json(make, target_name, backend_name, &artifact.name)?;

    let mut cmd = build_serialize_command(
        &script_abs,
        out,
        &config_file,
        context,
        target_name,
        &artifact.name,
    );

    let child = cmd.spawn().with_context(|| {
        format!(
            "spawning {} {}",
            script_info.script_name,
            script_abs.display()
        )
    })?;

    let output = run_command_with_timeout(child, script_info.script_name, SERIALIZATION_TIMEOUT)?;

    if !output.exit_success {
        bail!(
            "{} script failed with non-zero exit status",
            script_info.script_name
        );
    }

    Ok(output)
}

/// Run the shared_serialize script for a generated shared artifact.
///
/// Shared artifacts span multiple machines or users and require additional
/// context during serialization. This function builds machines.json and users.json
/// files containing backend configurations for each target, then executes the
/// `shared_serialize` script.
///
/// # Arguments
///
/// * `artifact_name` - Name of the shared artifact being serialized
/// * `backend_name` - Name of the backend (e.g., "agenix", "sops-nix")
/// * `backend` - The backend configuration with script paths
/// * `out` - Directory containing the generated files to serialize
/// * `make` - The make configuration for backend settings
/// * `nixos_targets` - List of NixOS machine names for this shared artifact
/// * `home_targets` - List of home-manager user names for this shared artifact
///
/// # Returns
///
/// Returns the captured output from the shared_serialize script.
///
/// # Errors
///
/// Returns an error if:
/// - The backend doesn't support shared artifacts (missing `shared_serialize` script)
/// - The script cannot be found or executed
/// - The script times out (after 30 seconds)
/// - The script exits with non-zero status
///
/// # JSON Files
///
/// The script receives paths to:
/// - `$machines` - JSON mapping machine names to their backend configs
/// - `$users` - JSON mapping user names to their backend configs
pub fn run_shared_serialize(
    artifact_name: &str,
    backend_name: &str,
    backend: &BackendConfiguration,
    out: &Path,
    make: &MakeConfiguration,
    nixos_targets: &[String],
    home_targets: &[String],
) -> Result<CapturedOutput> {
    let entry = backend.get_backend(&backend_name.to_string())?;
    let shared_ser = entry.shared_serialize.as_ref().ok_or_else(|| {
        anyhow::anyhow!(
            "backend '{}' does not support shared artifacts: missing 'shared_serialize'",
            backend_name
        )
    })?;

    let ser_abs = validate_backend_script(
        backend_name,
        "shared_serialize",
        &backend.base_path,
        shared_ser,
    )?;

    let (_, machines_file) = build_machines_json(make, nixos_targets, backend_name)?;
    let (_, users_file) = build_users_json(make, home_targets, backend_name)?;

    crate::log_debug!(
        "running shared_serialize: artifact=\"{}\" out=\"{}\" machines=\"{}\" users=\"{}\"",
        artifact_name,
        out.display(),
        machines_file.display(),
        users_file.display()
    );

    let mut cmd =
        build_shared_serialize_command(&ser_abs, out, &machines_file, &users_file, artifact_name);

    let child = cmd
        .spawn()
        .with_context(|| format!("spawning shared_serialize {}", ser_abs.display()))?;

    let output = run_command_with_timeout(child, "shared_serialize", SERIALIZATION_TIMEOUT)?;

    if !output.exit_success {
        bail!("shared_serialize script failed with non-zero exit status");
    }

    Ok(output)
}

/// Check if serialization is up to date for an artifact.
///
/// Runs the `check_serialization` script to determine if the artifact needs
/// regeneration. The script checks if the current serialized state matches
/// what would be generated from the current configuration and prompts.
///
/// # Behavior
///
/// The script uses exit codes to communicate status:
/// - Exit 0: Artifact is up-to-date (no regeneration needed)
/// - Exit non-zero: Artifact needs generation
///
/// # Arguments
///
/// * `artifact` - The artifact definition to check
/// * `target` - Name of the target (machine or username)
/// * `backend` - The backend configuration with script paths
/// * `make` - The make configuration for backend settings
/// * `context` - Context string: "nixos" or "homemanager"
///
/// # Returns
///
/// Returns a `CheckResult` containing:
/// - `needs_generation`: `true` if artifact should be regenerated
/// - `output`: Captured stdout/stderr from the script
///
/// # Errors
///
/// Returns an error if:
/// - The backend doesn't have a check_serialization script configured
/// - The script cannot be found or executed
/// - The script times out (after 30 seconds)
pub fn run_check_serialization(
    artifact: &ArtifactDef,
    target: &str,
    backend: &BackendConfiguration,
    make: &MakeConfiguration,
    context: &str,
) -> Result<CheckResult> {
    let backend_name = &artifact.serialization;
    let entry = backend.get_backend(backend_name)?;

    let script_info = get_check_script(&entry, context);
    let script_path = script_info.script_path.ok_or_else(|| {
        anyhow::anyhow!(
            "backend '{}' has no '{}' script configured",
            backend_name,
            script_info.script_name
        )
    })?;

    let inputs = TempFile::new_dir_with_name(&format!("inputs-{}", artifact.name))?;
    let (_, config_file) = build_config_json(make, target, backend_name, &artifact.name)?;

    write_check_input_files(artifact, &inputs, make)?;

    let script_abs = validate_backend_script(
        backend_name,
        script_info.script_name,
        &backend.base_path,
        script_path,
    )?;

    crate::log_debug!(
        "running {}: env inputs=\"{}\" {}=\"{}\" artifact=\"{}\" {}",
        script_info.script_name,
        inputs.display(),
        get_target_label(context),
        target,
        artifact.name,
        script_abs.display()
    );

    let mut cmd = build_check_command(
        &script_abs,
        &inputs,
        &config_file,
        context,
        target,
        &artifact.name,
    );

    let child = cmd.spawn().with_context(|| {
        format!(
            "spawning {} {}",
            script_info.script_name,
            script_abs.display()
        )
    })?;

    let result =
        run_with_captured_output_and_timeout(child, script_info.script_name, SERIALIZATION_TIMEOUT);
    handle_check_output(result, &artifact.name)
}

/// Check if shared serialization is up to date for a shared artifact.
///
/// Runs the `shared_check_serialization` script for shared artifacts that span
/// multiple machines or users. Similar to `run_check_serialization` but provides
/// machines.json and users.json context for the script to check all targets.
///
/// # Behavior
///
/// The script uses exit codes to communicate status:
/// - Exit 0: Artifact is up-to-date for all targets
/// - Exit non-zero: Artifact needs generation for at least one target
///
/// # Arguments
///
/// * `artifact_name` - Name of the shared artifact to check
/// * `backend_name` - Name of the backend (e.g., "agenix", "sops-nix")
/// * `backend` - The backend configuration with script paths
/// * `make` - The make configuration for backend settings
/// * `nixos_targets` - List of NixOS machine names for this shared artifact
/// * `home_targets` - List of home-manager user names for this shared artifact
///
/// # Returns
///
/// Returns a `CheckResult` containing:
/// - `needs_generation`: `true` if artifact should be regenerated
/// - `output`: Captured stdout/stderr from the script
///
/// # Errors
///
/// Returns an error if:
/// - The backend doesn't support shared artifacts (missing `shared_check_serialization`)
/// - The script cannot be found or executed
/// - The script times out (after 30 seconds)
pub fn run_shared_check_serialization(
    artifact_name: &str,
    backend_name: &str,
    backend: &BackendConfiguration,
    make: &MakeConfiguration,
    nixos_targets: &[String],
    home_targets: &[String],
) -> Result<CheckResult> {
    let entry = backend.get_backend(&backend_name.to_string())?;
    let check_script = entry.shared_check_serialization.as_ref().ok_or_else(|| {
        anyhow::anyhow!(
            "backend '{}' does not support shared artifacts: missing 'shared_check_serialization'",
            backend_name
        )
    })?;

    let script_abs = validate_backend_script(
        backend_name,
        "shared_check_serialization",
        &backend.base_path,
        check_script,
    )?;

    let (_, machines_file) = build_machines_json(make, nixos_targets, backend_name)?;
    let (_, users_file) = build_users_json(make, home_targets, backend_name)?;

    crate::log_debug!(
        "running shared_check_serialization: artifact=\"{}\" machines=\"{}\" users=\"{}\"",
        artifact_name,
        machines_file.display(),
        users_file.display()
    );

    let mut cmd =
        build_shared_check_command(&script_abs, &machines_file, &users_file, artifact_name);

    let child = cmd.spawn().with_context(|| {
        format!(
            "spawning shared_check_serialization {}",
            script_abs.display()
        )
    })?;

    let result = run_with_captured_output_and_timeout(
        child,
        "shared_check_serialization",
        SERIALIZATION_TIMEOUT,
    );
    handle_check_output(result, artifact_name)
}
