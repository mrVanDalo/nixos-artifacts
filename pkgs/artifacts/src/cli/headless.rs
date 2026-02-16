//! Headless mode for programmatic artifact generation.
//!
//! This module provides a non-interactive API for generating artifacts,
//! suitable for testing and automation. It bypasses the TUI and executes
//! effects directly, returning results that can be programmatically verified.

use crate::app::model::{ArtifactEntry, ArtifactStatus, StepLogs, TargetType};
use crate::backend::generator::{run_generator_script, verify_generated_files};
use crate::backend::serialization::run_serialize;
use crate::backend::tempfile::TempFile;
use crate::config::backend::BackendConfiguration;
use crate::config::make::{ArtifactDef, MakeConfiguration};
use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::path::PathBuf;

/// Result of generating a single artifact in headless mode.
#[derive(Debug)]
pub struct HeadlessArtifactResult {
    /// Target machine or user
    pub target: String,
    /// Artifact name
    pub artifact_name: String,
    /// Whether generation succeeded
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
    /// Generated file contents (file name -> content)
    /// Content is stored since temp directories are cleaned up
    pub generated_file_contents: BTreeMap<String, String>,
}

/// Result of a headless generation run.
#[derive(Debug)]
pub struct HeadlessResult {
    /// Results for each artifact
    pub artifacts: Vec<HeadlessArtifactResult>,
    /// Whether all artifacts succeeded
    pub all_succeeded: bool,
}

/// Diagnostic information captured during artifact generation.
///
/// This structure captures all relevant information during the generation process
/// for debugging and test failure investigation. It includes configuration,
/// environment variables, temporary file contents, and captured output from
/// generator and backend scripts.
#[derive(Debug, Clone)]
pub struct DiagnosticInfo {
    /// Name of the artifact being generated
    pub artifact_name: String,
    /// Target machine or user
    pub target: String,
    /// Backend configuration (serialized TOML)
    pub backend_config: String,
    /// Make configuration (serialized JSON)
    pub make_config: String,
    /// Environment variables during generation
    pub environment_vars: HashMap<String, String>,
    /// Contents of temporary input files
    pub temp_input_contents: HashMap<String, String>,
    /// Contents of temporary prompt files
    pub temp_prompt_contents: HashMap<String, String>,
    /// Generator stdout output
    pub generator_stdout: Option<String>,
    /// Generator stderr output
    pub generator_stderr: Option<String>,
    /// Backend serialize stdout output
    pub backend_stdout: Option<String>,
    /// Backend serialize stderr output
    pub backend_stderr: Option<String>,
    /// Paths to generated files
    pub generated_files: Vec<PathBuf>,
    /// Error message if generation failed
    pub error: Option<String>,
}

impl DiagnosticInfo {
    /// Create a new DiagnosticInfo with basic information.
    pub fn new(artifact_name: String, target: String) -> Self {
        Self {
            artifact_name,
            target,
            backend_config: String::new(),
            make_config: String::new(),
            environment_vars: HashMap::new(),
            temp_input_contents: HashMap::new(),
            temp_prompt_contents: HashMap::new(),
            generator_stdout: None,
            generator_stderr: None,
            backend_stdout: None,
            backend_stderr: None,
            generated_files: Vec::new(),
            error: None,
        }
    }

    /// Format diagnostic information into human-readable output.
    pub fn format(&self) -> String {
        let mut output = String::new();

        // Header
        output.push_str("═══════════════════════════════════════════════════════════\n");
        output.push_str(&format!("Diagnostic Report for: {}\n", self.artifact_name));
        output.push_str(&format!("Target: {}\n", self.target));
        output.push_str("═══════════════════════════════════════════════════════════\n\n");

        // Configuration section
        output.push_str("─── Configuration ───\n");
        output.push_str(&format!("Backend Config:\n{}\n\n", self.backend_config));
        output.push_str(&format!("Make Config:\n{}\n\n", self.make_config));

        // Environment section
        output.push_str("─── Environment Variables ───\n");
        let mut env_vars: Vec<_> = self.environment_vars.iter().collect();
        env_vars.sort_by(|a, b| a.0.cmp(b.0));
        for (key, value) in env_vars {
            // Skip sensitive variables
            let key_upper = key.to_uppercase();
            if key_upper.contains("SECRET")
                || key_upper.contains("PASSWORD")
                || key_upper.contains("TOKEN")
                || key_upper.contains("KEY")
            {
                output.push_str(&format!("{}: [REDACTED]\n", key));
            } else {
                output.push_str(&format!("{}={}\n", key, value));
            }
        }
        output.push('\n');

        // Input files section
        output.push_str("─── Input Files ───\n");
        if self.temp_input_contents.is_empty() {
            output.push_str("(no input files)\n");
        } else {
            for (name, content) in &self.temp_input_contents {
                output.push_str(&format!("\n--- {} ---\n", name));
                output.push_str(content);
                output.push('\n');
            }
        }
        output.push('\n');

        // Prompt files section
        output.push_str("─── Prompt Files ───\n");
        if self.temp_prompt_contents.is_empty() {
            output.push_str("(no prompt files - prompt values redacted)\n");
        } else {
            for (name, _content) in &self.temp_prompt_contents {
                output.push_str(&format!("{}: [REDACTED]\n", name));
            }
        }
        output.push('\n');

        // Generated files section
        output.push_str("─── Generated Files ───\n");
        if self.generated_files.is_empty() {
            output.push_str("(no files generated)\n");
        } else {
            for path in &self.generated_files {
                output.push_str(&format!("- {}\n", path.display()));
            }
        }
        output.push('\n');

        // Generator output section
        output.push_str("─── Generator Output ───\n");
        if let Some(stdout) = &self.generator_stdout {
            output.push_str(&format!("stdout:\n{}\n", stdout));
        } else {
            output.push_str("stdout: (not captured)\n");
        }
        if let Some(stderr) = &self.generator_stderr {
            output.push_str(&format!("stderr:\n{}\n", stderr));
        } else {
            output.push_str("stderr: (not captured)\n");
        }
        output.push('\n');

        // Backend output section
        output.push_str("─── Backend Output ───\n");
        if let Some(stdout) = &self.backend_stdout {
            output.push_str(&format!("stdout:\n{}\n", stdout));
        } else {
            output.push_str("stdout: (not captured)\n");
        }
        if let Some(stderr) = &self.backend_stderr {
            output.push_str(&format!("stderr:\n{}\n", stderr));
        } else {
            output.push_str("stderr: (not captured)\n");
        }
        output.push('\n');

        // Error section
        output.push_str("─── Error Information ───\n");
        if let Some(error) = &self.error {
            output.push_str(&format!("Error: {}\n", error));
        } else {
            output.push_str("(no error)\n");
        }

        // Footer
        output.push_str("\n═══════════════════════════════════════════════════════════\n");
        output.push_str("End of Diagnostic Report\n");
        output.push_str("═══════════════════════════════════════════════════════════\n");

        output
    }
}

/// Prompt values for headless generation.
/// Keys are prompt names, values are the user input.
pub type PromptValues = BTreeMap<String, String>;

/// Generate a single artifact in headless mode.
///
/// This executes the full pipeline:
/// 1. Run check_serialization to see if generation is needed
/// 2. If needed, run the generator with provided prompts
/// 3. Run serialize to store the artifact
///
/// # Arguments
/// * `target` - Machine or user name (e.g., "machine-one" or "alice@host")
/// * `artifact` - The artifact definition
/// * `prompt_values` - Values for each prompt (empty if no prompts)
/// * `backend` - The backend configuration
/// * `make_config` - The make configuration for paths
///
/// # Returns
/// The result of the generation attempt
pub fn generate_single_artifact(
    target: &str,
    artifact: &ArtifactDef,
    prompt_values: &PromptValues,
    backend: &BackendConfiguration,
    make_config: &MakeConfiguration,
) -> Result<HeadlessArtifactResult> {
    // For now, always assume generation is needed (fail-open behavior)
    // TODO: Implement proper check_serialization when backend supports it

    // Create temp directory for generation
    let temp_dir = TempFile::new_dir("artifacts-headless")
        .context("Failed to create temp directory for artifact generation")?;

    // Run generator with prompts
    let out_dir = temp_dir.path().join("out");
    std::fs::create_dir_all(&out_dir)?;

    // Write prompts to temp directory
    let prompts_dir = temp_dir.path().join("prompts");
    std::fs::create_dir_all(&prompts_dir)?;
    for (prompt_name, value) in prompt_values {
        let prompt_file = prompts_dir.join(prompt_name);
        std::fs::write(&prompt_file, value)
            .with_context(|| format!("Failed to write prompt '{}'", prompt_name))?;
    }

    // Run generator using the artifact's generator script
    let artifact_entry = ArtifactEntry {
        target: target.to_string(),
        target_type: TargetType::Nixos,
        artifact: artifact.clone(),
        status: ArtifactStatus::Pending,
        step_logs: StepLogs::default(),
    };

    let _ = run_generator_script(
        &artifact_entry.artifact,
        target,
        &make_config.make_base,
        &prompts_dir,
        &out_dir,
        "nixos", // context - assume nixos for now
    )
    .with_context(|| format!("Generator failed for artifact '{}'", artifact.name))?;

    // Verify generated files match expectations
    verify_generated_files(artifact, &out_dir).with_context(|| {
        format!(
            "Generated files verification failed for artifact '{}'",
            artifact.name
        )
    })?;

    // Collect generated file contents before temp directory is cleaned up
    let mut generated_file_contents = BTreeMap::new();
    for file_name in artifact.files.keys() {
        let file_path = out_dir.join(file_name);
        if file_path.exists() {
            match std::fs::read_to_string(&file_path) {
                Ok(content) => {
                    generated_file_contents.insert(file_name.clone(), content);
                }
                Err(e) => {
                    eprintln!(
                        "Warning: failed to read generated file {}: {}",
                        file_path.display(),
                        e
                    );
                }
            }
        }
    }

    // Run serialize - this stores the artifact in the backend
    let serialize_result = run_serialize(
        artifact,
        backend,
        &out_dir,
        target,
        make_config,
        "nixos", // context - assume nixos for now
    );

    let success = serialize_result.is_ok();
    let error = serialize_result.err().map(|e| e.to_string());

    Ok(HeadlessArtifactResult {
        target: target.to_string(),
        artifact_name: artifact.name.clone(),
        success,
        error,
        generated_file_contents,
    })
}

/// Generate a single artifact in headless mode with diagnostic capture.
///
/// This function has the same behavior as `generate_single_artifact` but captures
/// detailed diagnostic information throughout the process. It returns both the
/// generation result and a `DiagnosticInfo` structure containing all captured
/// information.
///
/// # Arguments
/// * `target` - Machine or user name (e.g., "machine-one" or "alice@host")
/// * `artifact` - The artifact definition
/// * `prompt_values` - Values for each prompt (empty if no prompts)
/// * `backend` - The backend configuration
/// * `make_config` - The make configuration for paths
///
/// # Returns
/// Tuple of (result, diagnostics):
/// - result: `Result<HeadlessArtifactResult>` - The generation result
/// - diagnostics: `DiagnosticInfo` - Captured diagnostic information (always populated)
///
/// # Example
/// ```rust
/// let (result, diagnostics) = generate_single_artifact_with_diagnostics(
///     "machine-name",
///     &artifact_def,
///     &prompt_values,
///     &backend,
///     &make_config,
/// )?;
///
/// if result.is_err() {
///     eprintln!("{}", diagnostics.format());
/// }
/// ```
pub fn generate_single_artifact_with_diagnostics(
    target: &str,
    artifact: &ArtifactDef,
    prompt_values: &PromptValues,
    backend: &BackendConfiguration,
    make_config: &MakeConfiguration,
) -> (Result<HeadlessArtifactResult>, DiagnosticInfo) {
    // Initialize diagnostic info
    let mut diagnostics = DiagnosticInfo::new(artifact.name.clone(), target.to_string());

    // Capture backend configuration
    match std::fs::read_to_string(backend.base_path.join("backend.toml")) {
        Ok(config) => diagnostics.backend_config = config,
        Err(e) => diagnostics.backend_config = format!("(failed to read: {})", e),
    }

    // Capture make configuration (basic info since MakeConfiguration doesn't implement Serialize)
    diagnostics.make_config = format!(
        "make_base: {}\nnixos_map keys: {:?}\nhome_map keys: {:?}",
        make_config.make_base.display(),
        make_config.nixos_map.keys().collect::<Vec<_>>(),
        make_config.home_map.keys().collect::<Vec<_>>()
    );

    // Capture environment variables
    for (key, value) in std::env::vars() {
        // Only capture relevant environment variables
        if key.starts_with("ARTIFACTS_") || key.starts_with("CARGO_") {
            diagnostics.environment_vars.insert(key, value);
        }
    }

    // Create temp directory for generation
    let temp_dir = match TempFile::new_dir("artifacts-headless") {
        Ok(dir) => dir,
        Err(e) => {
            diagnostics.error = Some(format!("Failed to create temp directory: {}", e));
            return (Err(e), diagnostics);
        }
    };

    // Run generator with prompts
    let out_dir = temp_dir.path().join("out");
    if let Err(e) = std::fs::create_dir_all(&out_dir) {
        diagnostics.error = Some(format!("Failed to create out directory: {}", e));
        return (Err(anyhow::anyhow!(e)), diagnostics);
    }

    // Write prompts to temp directory and capture
    let prompts_dir = temp_dir.path().join("prompts");
    if let Err(e) = std::fs::create_dir_all(&prompts_dir) {
        diagnostics.error = Some(format!("Failed to create prompts directory: {}", e));
        return (Err(anyhow::anyhow!(e)), diagnostics);
    }

    for (prompt_name, value) in prompt_values {
        let prompt_file = prompts_dir.join(prompt_name);
        // Don't capture actual prompt values (potentially sensitive)
        diagnostics
            .temp_prompt_contents
            .insert(prompt_name.clone(), "[REDACTED]".to_string());
        if let Err(e) = std::fs::write(&prompt_file, value) {
            diagnostics.error = Some(format!("Failed to write prompt '{}': {}", prompt_name, e));
            return (
                Err(anyhow::anyhow!(
                    "Failed to write prompt '{}': {}",
                    prompt_name,
                    e
                )),
                diagnostics,
            );
        }
    }

    // Run generator script with output capture
    let artifact_entry = ArtifactEntry {
        target: target.to_string(),
        target_type: TargetType::Nixos,
        artifact: artifact.clone(),
        status: ArtifactStatus::Pending,
        step_logs: StepLogs::default(),
    };

    let generator_result = run_generator_script(
        &artifact_entry.artifact,
        target,
        &make_config.make_base,
        &prompts_dir,
        &out_dir,
        "nixos",
    );

    // Note: run_generator_script currently doesn't capture output, so we
    // can't populate generator_stdout/stderr yet. In the future, this could
    // be enhanced to capture generator output.

    if let Err(e) = &generator_result {
        diagnostics.error = Some(format!("Generator failed: {}", e));
        return (
            Err(anyhow::anyhow!(
                "Generator failed for artifact '{}': {}",
                artifact.name,
                e
            )),
            diagnostics,
        );
    }

    // Verify generated files
    if let Err(e) = verify_generated_files(artifact, &out_dir) {
        diagnostics.error = Some(format!("Generated files verification failed: {}", e));
        return (
            Err(anyhow::anyhow!(
                "Generated files verification failed for artifact '{}': {}",
                artifact.name,
                e
            )),
            diagnostics,
        );
    }

    // Collect generated file contents and paths
    let mut generated_file_contents = BTreeMap::new();
    for file_name in artifact.files.keys() {
        let file_path = out_dir.join(file_name);
        if file_path.exists() {
            diagnostics.generated_files.push(file_path.clone());
            match std::fs::read_to_string(&file_path) {
                Ok(content) => {
                    generated_file_contents.insert(file_name.clone(), content);
                }
                Err(e) => {
                    eprintln!(
                        "Warning: failed to read generated file {}: {}",
                        file_path.display(),
                        e
                    );
                }
            }
        }
    }

    // Run serialize
    let serialize_result = run_serialize(artifact, backend, &out_dir, target, make_config, "nixos");

    // Note: run_serialize doesn't currently capture stdout/stderr, but we
    // could enhance it in the future to return CapturedOutput

    let success = serialize_result.is_ok();
    let error = serialize_result.as_ref().err().map(|e| e.to_string());

    if let Some(err) = &error {
        diagnostics.error = Some(err.clone());
    }

    let result = HeadlessArtifactResult {
        target: target.to_string(),
        artifact_name: artifact.name.clone(),
        success,
        error,
        generated_file_contents,
    };

    (Ok(result), diagnostics)
}

/// Generate all artifacts for a specific target in headless mode.
///
/// # Arguments
/// * `target` - Machine or user name
/// * `artifacts` - Map of artifact names to definitions
/// * `prompt_values` - Map of artifact names to their prompt values
/// * `backend` - Backend configuration
/// * `make_config` - Make configuration
///
/// # Returns
/// Results for all artifacts
pub fn generate_artifacts_for_target(
    target: &str,
    artifacts: &BTreeMap<String, ArtifactDef>,
    prompt_values: &BTreeMap<String, PromptValues>,
    backend: &BackendConfiguration,
    make_config: &MakeConfiguration,
) -> Result<Vec<HeadlessArtifactResult>> {
    let mut results = Vec::new();

    for (artifact_name, artifact) in artifacts {
        // Get prompt values for this artifact, or empty if none provided
        let artifact_prompts = prompt_values
            .get(artifact_name)
            .cloned()
            .unwrap_or_default();

        let result =
            generate_single_artifact(target, artifact, &artifact_prompts, backend, make_config)?;

        results.push(result);
    }

    Ok(results)
}

/// Generate all artifacts from a make configuration in headless mode.
///
/// # Arguments
/// * `backend` - Backend configuration
/// * `make_config` - Make configuration
/// * `prompt_values` - Map of target -> artifact name -> prompt name -> value
///
/// # Returns
/// Combined results for all targets and artifacts
pub fn generate_all_artifacts_headless(
    backend: &BackendConfiguration,
    make_config: &MakeConfiguration,
    prompt_values: &BTreeMap<String, BTreeMap<String, PromptValues>>,
) -> Result<HeadlessResult> {
    let mut all_results = Vec::new();

    // Generate for NixOS machines
    for (machine, artifacts) in &make_config.nixos_map {
        let machine_prompts = prompt_values.get(machine).cloned().unwrap_or_default();
        let results = generate_artifacts_for_target(
            machine,
            artifacts,
            &machine_prompts,
            backend,
            make_config,
        )?;
        all_results.extend(results);
    }

    // Generate for home-manager users
    for (user, artifacts) in &make_config.home_map {
        let user_prompts = prompt_values.get(user).cloned().unwrap_or_default();
        let results =
            generate_artifacts_for_target(user, artifacts, &user_prompts, backend, make_config)?;
        all_results.extend(results);
    }

    let all_succeeded = all_results.iter().all(|r| r.success);

    Ok(HeadlessResult {
        artifacts: all_results,
        all_succeeded,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests will be added in 06-02
}
