use crate::backend::helpers::{resolve_path, validate_backend_script};
use crate::backend::output_capture::{
    run_with_captured_output_and_timeout, CapturedOutput, ScriptError,
};
use crate::backend::tempfile::TempFile;
use crate::config::backend::{BackendConfiguration, BackendEntry};
use crate::config::make::{ArtifactDef, MakeConfiguration};
use crate::log_debug;
use anyhow::{bail, Context, Result};
use serde_json::{json, to_string_pretty, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

/// Timeout duration for serialization scripts (30 seconds)
const SERIALIZATION_TIMEOUT: Duration = Duration::from_secs(30);

/// Result of running check_serialization script
pub struct CheckResult {
    /// True if the artifact needs generation, false if up-to-date
    pub needs_generation: bool,
    /// Captured stdout/stderr from the script
    pub output: CapturedOutput,
}

/// Script information extracted from backend entry based on context
struct ScriptInfo<'a> {
    script_path: Option<&'a String>,
    script_name: &'a str,
}

/// Handle the output from a script execution, converting ScriptErrors to CheckResults
fn handle_check_output(
    result: Result<CapturedOutput, ScriptError>,
    artifact_name: &str,
) -> Result<CheckResult> {
    match result {
        Ok(out) => {
            let needs_generation = !out.exit_success;
            if out.exit_success {
                log_debug!("OK -> skipping generation");
            } else {
                log_debug!("needs generation");
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
            log_debug!(
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

/// Verify output succeeded, bail on failure
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

/// Run the serialize script for a generated artifact
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

/// Run the shared_serialize script for a generated shared artifact
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

    log_debug!(
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

/// Check if serialization is up to date
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

    let target_label = get_target_label(context);
    log_debug!(
        "running {}: env inputs=\"{}\" {}=\"{}\" artifact=\"{}\" {}",
        script_info.script_name,
        inputs.display(),
        target_label,
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

/// Check if shared serialization is up to date
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

    log_debug!(
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
