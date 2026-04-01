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

use crate::app::model::TargetType;
use crate::backend::helpers::{resolve_path, validate_backend_script};
use crate::backend::output_capture::{
    CapturedOutput, ScriptError, run_with_captured_output_and_timeout,
};
use crate::backend::tempfile::TempFile;
use crate::config::backend::{BackendConfiguration, BackendEntry};
use crate::config::make::{ArtifactDef, MakeConfiguration};
use crate::{log_debug, log_info};
use anyhow::{Context, Result, bail};
use serde_json::{Map, Value, json, to_string_pretty};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

/// Timeout duration for serialization scripts (30 seconds).
///
/// This timeout applies to all backend script executions to prevent
/// indefinite hanging on misconfigured scripts or slow backends.
const SERIALIZATION_TIMEOUT: Duration = Duration::from_secs(30);

// =============================================================================
// Unified Serialization Context
// =============================================================================

/// Describes what we're serializing and how to configure the environment.
///
/// This enum unifies the handling of single-target and shared artifacts,
/// allowing the same core logic to be used for both scenarios while
/// providing context-specific environment setup.
pub enum SerializationContext<'a> {
    /// Per-target artifact (one NixOS machine or one home-manager user).
    Single {
        artifact: &'a ArtifactDef,
        target_type: &'a TargetType,
    },
    /// Shared artifact across multiple machines/users.
    Shared {
        artifact_name: &'a str,
        backend_name: &'a str,
        nixos_targets: &'a [String],
        home_targets: &'a [String],
    },
}

/// Holds temp files needed during script execution.
/// Dropped automatically after the function returns, cleaning up temp files.
struct ConfigFiles {
    /// TempFile handles that must stay alive until the command finishes
    _handles: Vec<TempFile>,
    /// Paths to configuration files
    paths: ConfigPaths,
}

/// Paths to configuration files, varying by context type.
enum ConfigPaths {
    /// Single target: one config.json file
    Single { config: PathBuf },
    /// Shared: machines.json and users.json files
    Shared { machines: PathBuf, users: PathBuf },
}

impl<'a> SerializationContext<'a> {
    /// Get the backend name for this context.
    fn backend_name(&self) -> &str {
        match self {
            SerializationContext::Single { artifact, .. } => &artifact.serialization,
            SerializationContext::Shared { backend_name, .. } => backend_name,
        }
    }

    /// Get the artifact name for this context.
    fn artifact_name(&self) -> &str {
        match self {
            SerializationContext::Single { artifact, .. } => &artifact.name,
            SerializationContext::Shared { artifact_name, .. } => artifact_name,
        }
    }

    /// Returns (script_path, script_name) for the serialize operation.
    fn serialize_script_info(&self, entry: &BackendEntry) -> Result<(String, &'static str)> {
        match self {
            SerializationContext::Single { target_type, .. } => {
                let (script_opt, script_name) = match target_type {
                    TargetType::HomeManager { .. } => (
                        entry.serialize_script(crate::config::backend::TargetType::Home),
                        "home_serialize",
                    ),
                    TargetType::NixOS { .. } => (
                        entry.serialize_script(crate::config::backend::TargetType::NixOS),
                        "nixos_serialize",
                    ),
                };
                let script_path = script_opt.ok_or_else(|| {
                    anyhow::anyhow!(
                        "backend '{}' has no '{}' script configured",
                        self.backend_name(),
                        script_name
                    )
                })?;
                Ok((script_path.clone(), script_name))
            }
            SerializationContext::Shared { backend_name, .. } => {
                let script_opt = entry.serialize_script(crate::config::backend::TargetType::Shared);
                let script_path = script_opt.ok_or_else(|| {
                    anyhow::anyhow!(
                        "backend '{}' does not support shared artifacts: missing 'shared.serialize'",
                        backend_name
                    )
                })?;
                Ok((script_path.clone(), "shared_serialize"))
            }
        }
    }

    /// Returns (script_path, script_name) for the check operation.
    fn check_script_info(&self, entry: &BackendEntry) -> Result<(String, &'static str)> {
        match self {
            SerializationContext::Single { target_type, .. } => {
                let (script_opt, script_name) = match target_type {
                    TargetType::HomeManager { .. } => (
                        entry.check_script(crate::config::backend::TargetType::Home),
                        "home_check_serialization",
                    ),
                    TargetType::NixOS { .. } => (
                        entry.check_script(crate::config::backend::TargetType::NixOS),
                        "nixos_check_serialization",
                    ),
                };
                let script_path = script_opt.ok_or_else(|| {
                    anyhow::anyhow!(
                        "backend '{}' has no '{}' script configured",
                        self.backend_name(),
                        script_name
                    )
                })?;
                Ok((script_path.clone(), script_name))
            }
            SerializationContext::Shared { backend_name, .. } => {
                let script_opt = entry.check_script(crate::config::backend::TargetType::Shared);
                let script_path = script_opt.ok_or_else(|| {
                    anyhow::anyhow!(
                        "backend '{}' does not support shared artifacts: missing 'shared.check'",
                        backend_name
                    )
                })?;
                Ok((script_path.clone(), "shared_check_serialization"))
            }
        }
    }

    /// Builds configuration files and returns handles + paths.
    fn build_config_files(&self, make: &MakeConfiguration) -> Result<ConfigFiles> {
        match self {
            SerializationContext::Single {
                artifact,
                target_type,
            } => {
                let target_name = target_type.target_name();
                let backend_name = &artifact.serialization;
                let (config_dir, config_path) =
                    build_config_json(make, target_name, backend_name, &artifact.name)?;
                Ok(ConfigFiles {
                    _handles: vec![config_dir],
                    paths: ConfigPaths::Single {
                        config: config_path,
                    },
                })
            }
            SerializationContext::Shared {
                backend_name,
                nixos_targets,
                home_targets,
                ..
            } => {
                let (machines_dir, machines_path) =
                    build_machines_json(make, nixos_targets, backend_name)?;
                let (users_dir, users_path) = build_users_json(make, home_targets, backend_name)?;
                Ok(ConfigFiles {
                    _handles: vec![machines_dir, users_dir],
                    paths: ConfigPaths::Shared {
                        machines: machines_path,
                        users: users_path,
                    },
                })
            }
        }
    }

    /// Applies context-specific env vars to a Command.
    fn apply_env(&self, cmd: &mut Command, config: &ConfigFiles) {
        cmd.env("artifact", self.artifact_name());

        match (&self, &config.paths) {
            (
                SerializationContext::Single { target_type, .. },
                ConfigPaths::Single {
                    config: config_path,
                },
            ) => {
                cmd.env("config", config_path);
                cmd.env("artifact_context", target_type.context_str());
                match target_type {
                    TargetType::HomeManager { username } => {
                        cmd.env("username", username);
                    }
                    TargetType::NixOS { machine } => {
                        cmd.env("machine", machine);
                    }
                }
            }
            (SerializationContext::Shared { .. }, ConfigPaths::Shared { machines, users }) => {
                cmd.env("machines", machines);
                cmd.env("users", users);
            }
            // These cases shouldn't happen due to how we construct things,
            // but handle gracefully by doing nothing extra
            _ => {}
        }
    }

    /// Log the environment for serialize operations.
    #[allow(unused_variables)]
    fn log_serialize_env(
        &self,
        script_name: &str,
        script_path: &Path,
        out: &Path,
        config: &ConfigFiles,
    ) {
        log_debug!(
            "running {}: script=\"{}\"",
            script_name,
            script_path.display()
        );

        match (&self, &config.paths) {
            (
                SerializationContext::Single { target_type, .. },
                ConfigPaths::Single {
                    config: config_path,
                },
            ) => {
                log_debug!(
                    "  environment: out=\"{}\" config=\"{}\" artifact=\"{}\" artifact_context=\"{}\"",
                    out.display(),
                    config_path.display(),
                    self.artifact_name(),
                    target_type.context_str()
                );
                match target_type {
                    TargetType::HomeManager { username } => {
                        log_debug!("  environment: username=\"{}\"", username);
                    }
                    TargetType::NixOS { machine } => {
                        log_debug!("  environment: machine=\"{}\"", machine);
                    }
                }
            }
            (SerializationContext::Shared { .. }, ConfigPaths::Shared { machines, users }) => {
                log_debug!(
                    "  environment: artifact=\"{}\" out=\"{}\" machines=\"{}\" users=\"{}\"",
                    self.artifact_name(),
                    out.display(),
                    machines.display(),
                    users.display()
                );
            }
            _ => {}
        }
    }

    /// Log the environment for check operations.
    #[allow(unused_variables)]
    fn log_check_env(
        &self,
        script_name: &str,
        script_path: &Path,
        inputs: Option<&Path>,
        config: &ConfigFiles,
    ) {
        log_debug!(
            "running {}: script=\"{}\"",
            script_name,
            script_path.display()
        );

        match (&self, &config.paths) {
            (
                SerializationContext::Single { target_type, .. },
                ConfigPaths::Single {
                    config: config_path,
                },
            ) => {
                if let Some(inputs_path) = inputs {
                    log_debug!(
                        "  environment: inputs=\"{}\" config=\"{}\" artifact=\"{}\" artifact_context=\"{}\"",
                        inputs_path.display(),
                        config_path.display(),
                        self.artifact_name(),
                        target_type.context_str()
                    );
                }
                match target_type {
                    TargetType::HomeManager { username } => {
                        log_debug!("  environment: username=\"{}\"", username);
                    }
                    TargetType::NixOS { machine } => {
                        log_debug!("  environment: machine=\"{}\"", machine);
                    }
                }
            }
            (SerializationContext::Shared { .. }, ConfigPaths::Shared { machines, users }) => {
                log_debug!(
                    "  environment: artifact=\"{}\" machines=\"{}\" users=\"{}\"",
                    self.artifact_name(),
                    machines.display(),
                    users.display()
                );
            }
            _ => {}
        }
    }
}

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
            stdout: _,
            stderr: _,
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
        Err(ScriptError::Failed { stdout, stderr, .. }) => Ok(make_failed_result(stdout, stderr)),
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

// =============================================================================
// Unified Command Builders
// =============================================================================

/// Build a Command for serialize script execution (unified for single and shared).
fn build_serialize_command_unified(
    script_path: &Path,
    out_dir: &Path,
    ctx: &SerializationContext<'_>,
    config: &ConfigFiles,
    log_level: crate::logging::LogLevel,
) -> Command {
    let mut cmd = Command::new("sh");
    cmd.arg(script_path)
        .env("out", out_dir)
        .env("LOG_LEVEL", log_level.as_str())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    ctx.apply_env(&mut cmd, config);

    #[cfg(test)]
    if let Ok(test_output_dir) = std::env::var("ARTIFACTS_TEST_OUTPUT_DIR") {
        cmd.env("ARTIFACTS_TEST_OUTPUT_DIR", test_output_dir);
    }

    cmd
}

/// Build a Command for check_serialization script execution (unified for single and shared).
fn build_check_command_unified(
    script_path: &Path,
    inputs_dir: Option<&Path>,
    ctx: &SerializationContext<'_>,
    config: &ConfigFiles,
    log_level: crate::logging::LogLevel,
) -> Command {
    let mut cmd = Command::new(script_path);
    cmd.env("LOG_LEVEL", log_level.as_str())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if let Some(inputs) = inputs_dir {
        cmd.env("inputs", inputs);
    }

    ctx.apply_env(&mut cmd, config);

    cmd
}

// =============================================================================
// Unified Inner Functions
// =============================================================================

/// Internal implementation for serialize operations (both single and shared).
fn run_serialize_inner(
    ctx: &SerializationContext<'_>,
    backend: &BackendConfiguration,
    out: &Path,
    make: &MakeConfiguration,
    log_level: crate::logging::LogLevel,
) -> Result<CapturedOutput> {
    let backend_name = ctx.backend_name();
    let entry = backend.get_backend(&backend_name.to_string())?;

    let (script_path, script_name) = ctx.serialize_script_info(&entry)?;
    let script_abs =
        validate_backend_script(backend_name, script_name, &backend.base_path, &script_path)?;

    let config = ctx.build_config_files(make)?;

    ctx.log_serialize_env(script_name, &script_abs, out, &config);

    let mut cmd = build_serialize_command_unified(&script_abs, out, ctx, &config, log_level);

    let child = cmd
        .spawn()
        .with_context(|| format!("spawning {} {}", script_name, script_abs.display()))?;

    let output = run_command_with_timeout(child, script_name, SERIALIZATION_TIMEOUT)?;

    if !output.exit_success {
        bail!("{} script failed with non-zero exit status", script_name);
    }

    log_info!(
        "{} output for '{}':\n{}",
        script_name,
        ctx.artifact_name(),
        output
    );

    Ok(output)
}

/// Internal implementation for check_serialization operations (both single and shared).
fn run_check_inner(
    ctx: &SerializationContext<'_>,
    backend: &BackendConfiguration,
    make: &MakeConfiguration,
    inputs_dir: Option<&TempFile>,
    log_level: crate::logging::LogLevel,
) -> Result<CheckResult> {
    let backend_name = ctx.backend_name();
    let entry = backend.get_backend(&backend_name.to_string())?;

    let (script_path, script_name) = ctx.check_script_info(&entry)?;
    let script_abs =
        validate_backend_script(backend_name, script_name, &backend.base_path, &script_path)?;

    let config = ctx.build_config_files(make)?;

    let inputs_path = inputs_dir.map(|d| d.as_ref() as &Path);
    ctx.log_check_env(script_name, &script_abs, inputs_path, &config);

    let mut cmd = build_check_command_unified(&script_abs, inputs_path, ctx, &config, log_level);

    let child = cmd
        .spawn()
        .with_context(|| format!("spawning {} {}", script_name, script_abs.display()))?;

    let result = run_with_captured_output_and_timeout(child, script_name, SERIALIZATION_TIMEOUT);

    let check_result = handle_check_output(result, ctx.artifact_name())?;

    log_info!(
        "{} output for '{}':\n{}",
        script_name,
        ctx.artifact_name(),
        check_result.output
    );

    Ok(check_result)
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
            stdout: _,
            stderr: _,
        }) => {
            bail!("{} timed out after {} seconds", name, timeout_secs);
        }
        Err(ScriptError::Io { message }) => {
            bail!("I/O error during {}: {}", script_name, message);
        }
        Err(ScriptError::Failed {
            exit_code,
            stdout: _,
            stderr,
        }) => {
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
    let output = CapturedOutput {
        stdout: Vec::new(),
        stderr: vec![format!(
            "{} timed out after {} seconds",
            script_name, timeout_secs
        )],
        exit_success: false,
    };
    CheckResult {
        needs_generation: true,
        output,
    }
}

/// Create CheckResult for I/O error
fn make_io_result(message: &str) -> CheckResult {
    let output = CapturedOutput {
        stdout: Vec::new(),
        stderr: vec![format!("I/O error: {}", message)],
        exit_success: false,
    };
    CheckResult {
        needs_generation: true,
        output,
    }
}

/// Create CheckResult for failed script execution
fn make_failed_result(stdout: String, stderr: String) -> CheckResult {
    let output = CapturedOutput {
        stdout: if stdout.is_empty() {
            Vec::new()
        } else {
            vec![stdout]
        },
        stderr: if stderr.is_empty() {
            Vec::new()
        } else {
            vec![stderr]
        },
        exit_success: false,
    };
    CheckResult {
        needs_generation: true,
        output,
    }
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
/// * `target_type` - Target type with name (Nixos { machine } or HomeManager { username })
/// * `make` - The make configuration for backend settings
/// * `log_level` - Log level string to pass to the script
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
/// For `HomeManager`: Sets `$username` environment variable
/// For `Nixos`: Sets `$machine` environment variable
pub fn run_serialize(
    artifact: &ArtifactDef,
    backend: &BackendConfiguration,
    out: &Path,
    target_type: &TargetType,
    make: &MakeConfiguration,
    log_level: crate::logging::LogLevel,
) -> Result<CapturedOutput> {
    let ctx = SerializationContext::Single {
        artifact,
        target_type,
    };
    run_serialize_inner(&ctx, backend, out, make, log_level)
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
/// * `log_level` - Log level to pass to the script
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
#[allow(clippy::too_many_arguments)]
pub fn run_shared_serialize(
    artifact_name: &str,
    backend_name: &str,
    backend: &BackendConfiguration,
    out: &Path,
    make: &MakeConfiguration,
    nixos_targets: &[String],
    home_targets: &[String],
    log_level: crate::logging::LogLevel,
) -> Result<CapturedOutput> {
    let ctx = SerializationContext::Shared {
        artifact_name,
        backend_name,
        nixos_targets,
        home_targets,
    };
    run_serialize_inner(&ctx, backend, out, make, log_level)
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
/// * `target_type` - Target type with name (Nixos { machine } or HomeManager { username })
/// * `backend` - The backend configuration with script paths
/// * `make` - The make configuration for backend settings
/// * `log_level` - Log level to pass to the script
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
    target_type: &TargetType,
    backend: &BackendConfiguration,
    make: &MakeConfiguration,
    log_level: crate::logging::LogLevel,
) -> Result<CheckResult> {
    // Create inputs directory and write input files for single-target check
    let inputs = TempFile::new_dir_with_name(&format!("inputs-{}", artifact.name))?;
    write_check_input_files(artifact, &inputs, make)?;

    let ctx = SerializationContext::Single {
        artifact,
        target_type,
    };
    run_check_inner(&ctx, backend, make, Some(&inputs), log_level)
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
/// * `log_level` - Log level to pass to the script
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
/// - The backend doesn't support shared artifacts (missing `shared.check`)
/// - The script cannot be found or executed
/// - The script times out (after 30 seconds)
pub fn run_shared_check_serialization(
    artifact_name: &str,
    backend_name: &str,
    backend: &BackendConfiguration,
    make: &MakeConfiguration,
    nixos_targets: &[String],
    home_targets: &[String],
    log_level: crate::logging::LogLevel,
) -> Result<CheckResult> {
    let ctx = SerializationContext::Shared {
        artifact_name,
        backend_name,
        nixos_targets,
        home_targets,
    };
    // Shared check doesn't use inputs directory
    run_check_inner(&ctx, backend, make, None, log_level)
}
