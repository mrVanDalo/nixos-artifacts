use crate::backend::helpers::{resolve_path, validate_backend_script};
use crate::backend::output_capture::{
    run_with_captured_output_and_timeout, CapturedOutput, ScriptError,
};
use crate::backend::tempfile::TempFile;
use crate::config::backend::BackendConfiguration;
use crate::config::make::{ArtifactDef, MakeConfiguration};
use anyhow::{bail, Context, Result};
use log::debug;
use serde_json::{json, to_string_pretty};
use std::fs;
use std::path::Path;
use std::process::Stdio;
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

/// Run the serialize script for a generated artifact.
///
/// This function resolves the serialize script path from the backend
/// configuration based on context (nixos or homemanager) and invokes it
/// with the appropriate environment variables:
/// - out: path to the generator output directory
/// - machine/username: the target name
/// - artifact: the artifact name
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

    // Select script based on context
    let (serialize_script, script_name) = if context == "homemanager" {
        (entry.home_serialize.as_ref(), "home_serialize")
    } else {
        (entry.nixos_serialize.as_ref(), "nixos_serialize")
    };

    let serialize_script = serialize_script.ok_or_else(|| {
        anyhow::anyhow!(
            "backend '{}' has no '{}' script configured",
            backend_name,
            script_name
        )
    })?;
    let ser_abs = validate_backend_script(
        backend_name,
        script_name,
        &backend.base_path,
        serialize_script,
    )?;

    // Create config file for the selected backend and target
    let config_dir = TempFile::new_dir("config")?;
    let config_file = config_dir.join("config.json");
    let config_value = make
        .get_backend_config_for(target_name, backend_name)
        .map(|m| serde_json::to_value(m).unwrap_or(serde_json::json!({})))
        .unwrap_or(serde_json::json!({}));
    let config_text = to_string_pretty(&config_value)?;
    fs::write(&config_file, &config_text)
        .with_context(|| format!("writing {}", config_file.display()))?;

    let mut cmd = std::process::Command::new("sh");
    cmd.arg(&ser_abs)
        .env("out", out)
        .env("config", &config_file)
        .env("artifact_context", context)
        .env("artifact", &artifact.name)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if context == "homemanager" {
        cmd.env("username", target_name);
    } else {
        cmd.env("machine", target_name);
    }

    // Pass through test output directory if set (only in test builds)
    #[cfg(test)]
    if let Ok(test_output_dir) = std::env::var("ARTIFACTS_TEST_OUTPUT_DIR") {
        cmd.env("ARTIFACTS_TEST_OUTPUT_DIR", test_output_dir);
    }

    let child = cmd
        .spawn()
        .with_context(|| format!("spawning {} {}", script_name, ser_abs.display()))?;

    let output =
        match run_with_captured_output_and_timeout(child, script_name, SERIALIZATION_TIMEOUT) {
            Ok(out) => out,
            Err(ScriptError::Timeout {
                script_name: name,
                timeout_secs,
            }) => {
                debug!(
                    "{} timed out after {}s for {}",
                    name, timeout_secs, artifact.name
                );
                bail!("{} timed out after {} seconds", name, timeout_secs);
            }
            Err(ScriptError::Io { message }) => {
                bail!("I/O error during serialization: {}", message);
            }
            Err(ScriptError::Failed { exit_code, stderr }) => {
                bail!(
                    "{} failed with exit code {}: {}",
                    script_name,
                    exit_code,
                    stderr
                );
            }
        };

    if !output.exit_success {
        bail!("{} script failed with non-zero exit status", script_name);
    }

    Ok(output)
}

/// Run the shared_serialize script for a generated shared artifact.
///
/// This function invokes the shared_serialize script with environment variables:
/// - artifact: the artifact name
/// - out: path to the generator output directory
/// - machines: path to JSON file mapping machine names to their backend configs
/// - users: path to JSON file mapping user identifiers to their backend configs
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

    // Create machines JSON file
    let machines_dir = TempFile::new_dir("machines")?;
    let machines_file = machines_dir.join("machines.json");
    let machines_config: serde_json::Map<String, serde_json::Value> = nixos_targets
        .iter()
        .map(|machine| {
            let config = make
                .get_backend_config_for(machine, backend_name)
                .map(|m| serde_json::to_value(m).unwrap_or(json!({})))
                .unwrap_or(json!({}));
            (machine.clone(), config)
        })
        .collect();
    let machines_text = to_string_pretty(&machines_config)?;
    fs::write(&machines_file, &machines_text)
        .with_context(|| format!("writing {}", machines_file.display()))?;

    // Create users JSON file
    let users_dir = TempFile::new_dir("users")?;
    let users_file = users_dir.join("users.json");
    let users_config: serde_json::Map<String, serde_json::Value> = home_targets
        .iter()
        .map(|user| {
            let config = make
                .get_backend_config_for(user, backend_name)
                .map(|m| serde_json::to_value(m).unwrap_or(json!({})))
                .unwrap_or(json!({}));
            (user.clone(), config)
        })
        .collect();
    let users_text = to_string_pretty(&users_config)?;
    fs::write(&users_file, &users_text)
        .with_context(|| format!("writing {}", users_file.display()))?;

    debug!(
        "running shared_serialize: artifact=\"{}\" out=\"{}\" machines=\"{}\" users=\"{}\"",
        artifact_name,
        out.display(),
        machines_file.display(),
        users_file.display()
    );

    let mut cmd = std::process::Command::new("sh");
    cmd.arg(&ser_abs)
        .env("artifact", artifact_name)
        .env("out", out)
        .env("machines", &machines_file)
        .env("users", &users_file)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // Pass through test output directory if set (only in test builds)
    #[cfg(test)]
    if let Ok(test_output_dir) = std::env::var("ARTIFACTS_TEST_OUTPUT_DIR") {
        cmd.env("ARTIFACTS_TEST_OUTPUT_DIR", test_output_dir);
    }

    let child = cmd
        .spawn()
        .with_context(|| format!("spawning shared_serialize {}", ser_abs.display()))?;

    let output = match run_with_captured_output_and_timeout(
        child,
        "shared_serialize",
        SERIALIZATION_TIMEOUT,
    ) {
        Ok(out) => out,
        Err(ScriptError::Timeout {
            script_name: name,
            timeout_secs,
        }) => {
            debug!(
                "{} timed out after {}s for {}",
                name, timeout_secs, artifact_name
            );
            bail!("{} timed out after {} seconds", name, timeout_secs);
        }
        Err(ScriptError::Io { message }) => {
            bail!("I/O error during shared_serialize: {}", message);
        }
        Err(ScriptError::Failed { exit_code, stderr }) => {
            bail!(
                "shared_serialize failed with exit code {}: {}",
                exit_code,
                stderr
            );
        }
    };

    if !output.exit_success {
        bail!("shared_serialize script failed with non-zero exit status");
    }

    Ok(output)
}

/// Check if serialization is up to date.
///
/// Returns CheckResult containing:
/// - needs_generation: true if regeneration is needed, false if up-to-date
/// - output: captured stdout/stderr from the check script
pub fn run_check_serialization(
    artifact: &ArtifactDef,
    target: &str,
    backend: &BackendConfiguration,
    make: &MakeConfiguration,
    context: &str,
) -> Result<CheckResult> {
    let backend_name = &artifact.serialization;
    let backend_entry = backend.get_backend(backend_name)?;

    // Select script based on context
    let (check_script, script_name) = if context == "homemanager" {
        (
            backend_entry.home_check_serialization.as_ref(),
            "home_check_serialization",
        )
    } else {
        (
            backend_entry.nixos_check_serialization.as_ref(),
            "nixos_check_serialization",
        )
    };

    let check_script = check_script.ok_or_else(|| {
        anyhow::anyhow!(
            "backend '{}' has no '{}' script configured",
            backend_name,
            script_name
        )
    })?;

    let inputs = TempFile::new_dir_with_name(&format!("inputs-{}", artifact.name))?;

    // Prepare backend config file
    let config_dir = TempFile::new_dir_with_name(&format!("config-{}", artifact.name))?;
    let config_file = config_dir.join("config.json");
    let config_value = make
        .get_backend_config_for(target, backend_name)
        .map(|m| serde_json::to_value(m).unwrap_or(json!({})))
        .unwrap_or(json!({}));
    let config_text = to_string_pretty(&config_value)?;
    fs::write(&config_file, &config_text)
        .with_context(|| format!("writing {}", config_file.display()))?;

    // Write input files
    for file in artifact.files.values() {
        let resolved_path = file
            .path
            .as_ref()
            .map(|path| resolve_path(&make.make_base, path));
        let json_path = inputs.join(&file.name);

        let text = to_string_pretty(&json!({
            "path": resolved_path,
            "owner": file.owner,
            "group": file.group,
        }))?;

        fs::write(&json_path, text).with_context(|| format!("writing {}", json_path.display()))?;
    }

    let check_abs =
        validate_backend_script(backend_name, script_name, &backend.base_path, check_script)?;

    let target_label = if context == "homemanager" {
        "username"
    } else {
        "machine"
    };
    debug!(
        "running {}: env inputs=\"{}\" {}=\"{}\" artifact=\"{}\" {}",
        script_name,
        inputs.display(),
        target_label,
        target,
        artifact.name,
        check_abs.display()
    );

    let mut cmd = std::process::Command::new(&check_abs);
    cmd.env("inputs", &*inputs)
        .env("config", &config_file)
        .env("artifact_context", context)
        .env("artifact", &artifact.name)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if context == "homemanager" {
        cmd.env("username", target);
    } else {
        cmd.env("machine", target);
    }

    let child = cmd
        .spawn()
        .with_context(|| format!("spawning {} {}", script_name, check_abs.display()))?;

    let output =
        match run_with_captured_output_and_timeout(child, script_name, SERIALIZATION_TIMEOUT) {
            Ok(out) => out,
            Err(ScriptError::Timeout {
                script_name: name,
                timeout_secs,
            }) => {
                debug!(
                    "{} timed out after {}s for {}",
                    name, timeout_secs, artifact.name
                );
                // Fail open - assume generation needed on timeout
                let mut timeout_output = CapturedOutput::default();
                timeout_output
                    .lines
                    .push(crate::backend::output_capture::OutputLine {
                        stream: crate::backend::output_capture::OutputStream::Stderr,
                        content: format!("{} timed out after {} seconds", name, timeout_secs),
                    });
                return Ok(CheckResult {
                    needs_generation: true,
                    output: timeout_output,
                });
            }
            Err(ScriptError::Io { message }) => {
                // Fail open - assume generation needed on error
                let mut error_output = CapturedOutput::default();
                error_output
                    .lines
                    .push(crate::backend::output_capture::OutputLine {
                        stream: crate::backend::output_capture::OutputStream::Stderr,
                        content: format!("I/O error: {}", message),
                    });
                return Ok(CheckResult {
                    needs_generation: true,
                    output: error_output,
                });
            }
            Err(ScriptError::Failed {
                exit_code: _,
                stderr,
            }) => {
                // Non-zero exit is expected when generation is needed
                let mut failed_output = CapturedOutput::default();
                failed_output
                    .lines
                    .push(crate::backend::output_capture::OutputLine {
                        stream: crate::backend::output_capture::OutputStream::Stderr,
                        content: stderr,
                    });
                return Ok(CheckResult {
                    needs_generation: true,
                    output: failed_output,
                });
            }
        };

    let needs_generation = !output.exit_success;

    if output.exit_success {
        debug!("{}: OK -> skipping generation", script_name);
    } else {
        debug!("{}: needs generation", script_name);
    }

    Ok(CheckResult {
        needs_generation,
        output,
    })
}

/// Check if shared serialization is up to date.
///
/// Returns CheckResult containing:
/// - needs_generation: true if regeneration is needed, false if up-to-date
/// - output: captured stdout/stderr from the check script
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

    let check_abs = validate_backend_script(
        backend_name,
        "shared_check_serialization",
        &backend.base_path,
        check_script,
    )?;

    // Create machines JSON file
    let machines_dir = TempFile::new_dir("machines")?;
    let machines_file = machines_dir.join("machines.json");
    let machines_config: serde_json::Map<String, serde_json::Value> = nixos_targets
        .iter()
        .map(|machine| {
            let config = make
                .get_backend_config_for(machine, backend_name)
                .map(|m| serde_json::to_value(m).unwrap_or(json!({})))
                .unwrap_or(json!({}));
            (machine.clone(), config)
        })
        .collect();
    let machines_text = to_string_pretty(&machines_config)?;
    fs::write(&machines_file, &machines_text)
        .with_context(|| format!("writing {}", machines_file.display()))?;

    // Create users JSON file
    let users_dir = TempFile::new_dir("users")?;
    let users_file = users_dir.join("users.json");
    let users_config: serde_json::Map<String, serde_json::Value> = home_targets
        .iter()
        .map(|user| {
            let config = make
                .get_backend_config_for(user, backend_name)
                .map(|m| serde_json::to_value(m).unwrap_or(json!({})))
                .unwrap_or(json!({}));
            (user.clone(), config)
        })
        .collect();
    let users_text = to_string_pretty(&users_config)?;
    fs::write(&users_file, &users_text)
        .with_context(|| format!("writing {}", users_file.display()))?;

    debug!(
        "running shared_check_serialization: artifact=\"{}\" machines=\"{}\" users=\"{}\"",
        artifact_name,
        machines_file.display(),
        users_file.display()
    );

    let mut cmd = std::process::Command::new(&check_abs);
    cmd.env("artifact", artifact_name)
        .env("machines", &machines_file)
        .env("users", &users_file)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let child = cmd.spawn().with_context(|| {
        format!(
            "spawning shared_check_serialization {}",
            check_abs.display()
        )
    })?;

    let output = match run_with_captured_output_and_timeout(
        child,
        "shared_check_serialization",
        SERIALIZATION_TIMEOUT,
    ) {
        Ok(out) => out,
        Err(ScriptError::Timeout {
            script_name: name,
            timeout_secs,
        }) => {
            debug!(
                "{} timed out after {}s for {}",
                name, timeout_secs, artifact_name
            );
            // Fail open - assume generation needed on timeout
            let mut timeout_output = CapturedOutput::default();
            timeout_output
                .lines
                .push(crate::backend::output_capture::OutputLine {
                    stream: crate::backend::output_capture::OutputStream::Stderr,
                    content: format!("{} timed out after {} seconds", name, timeout_secs),
                });
            return Ok(CheckResult {
                needs_generation: true,
                output: timeout_output,
            });
        }
        Err(ScriptError::Io { message }) => {
            // Fail open - assume generation needed on error
            let mut error_output = CapturedOutput::default();
            error_output
                .lines
                .push(crate::backend::output_capture::OutputLine {
                    stream: crate::backend::output_capture::OutputStream::Stderr,
                    content: format!("I/O error: {}", message),
                });
            return Ok(CheckResult {
                needs_generation: true,
                output: error_output,
            });
        }
        Err(ScriptError::Failed {
            exit_code: _,
            stderr,
        }) => {
            // Non-zero exit is expected when generation is needed
            let mut failed_output = CapturedOutput::default();
            failed_output
                .lines
                .push(crate::backend::output_capture::OutputLine {
                    stream: crate::backend::output_capture::OutputStream::Stderr,
                    content: stderr,
                });
            return Ok(CheckResult {
                needs_generation: true,
                output: failed_output,
            });
        }
    };

    let needs_generation = !output.exit_success;

    if output.exit_success {
        debug!("shared_check_serialization: OK -> skipping generation");
    } else {
        debug!("shared_check_serialization: needs generation");
    }

    Ok(CheckResult {
        needs_generation,
        output,
    })
}
