//! Common test utilities for artifact generation tests.
//!
//! This module provides a unified test harness that uses the actual TUI code path
//! (BackendEffectHandler + Model + Effects) for testing, ensuring that tests
//! exercise the real production code paths rather than a separate headless API.

use anyhow::{Context, Result};
use artifacts::app::effect::Effect;
use artifacts::app::message::Message;
use artifacts::app::model::{ArtifactEntry, ArtifactStatus, ListEntry, Model, StepLogs, TargetType};
use artifacts::config::backend::BackendConfiguration;
use artifacts::config::make::{ArtifactDef, MakeConfiguration};
use artifacts::config::nix::build_make_from_flake;
use artifacts::tui::BackendEffectHandler;
use artifacts::tui::EffectHandler;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use tempfile::TempDir;

/// Result of artifact generation for tests.
#[derive(Debug)]
pub struct TestArtifactResult {
    /// Target machine or user
    pub target: String,
    /// Artifact name
    pub artifact_name: String,
    /// Whether generation succeeded
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
    /// Generated file contents (file name -> content)
    pub generated_file_contents: BTreeMap<String, String>,
}

/// Diagnostic information captured during artifact generation.
///
/// This structure captures all relevant information during the generation process
/// for debugging and test failure investigation.
#[derive(Debug, Clone)]
pub struct DiagnosticInfo {
    /// Name of the artifact being generated
    pub artifact_name: String,
    /// Target machine or user
    pub target: String,
    /// Backend configuration (serialized TOML)
    pub backend_config: String,
    /// Make configuration (serialized JSON string or summary)
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

        output.push_str("═══════════════════════════════════════════════════════════\n");
        output.push_str(&format!("Diagnostic Report for: {}\n", self.artifact_name));
        output.push_str(&format!("Target: {}\n", self.target));
        output.push_str("═══════════════════════════════════════════════════════════\n\n");

        output.push_str("─── Configuration ───\n");
        output.push_str(&format!("Backend Config:\n{}\n\n", self.backend_config));
        output.push_str(&format!("Make Config:\n{}\n\n", self.make_config));

        output.push_str("─── Environment Variables ───\n");
        let mut env_vars: Vec<_> = self.environment_vars.iter().collect();
        env_vars.sort_by(|a, b| a.0.cmp(b.0));
        for (key, value) in env_vars {
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

        output.push_str("─── Prompt Files ───\n");
        if self.temp_prompt_contents.is_empty() {
            output.push_str("(no prompt files - prompt values redacted)\n");
        } else {
            for name in self.temp_prompt_contents.keys() {
                output.push_str(&format!("{}: [REDACTED]\n", name));
            }
        }
        output.push('\n');

        output.push_str("─── Generated Files ───\n");
        if self.generated_files.is_empty() {
            output.push_str("(no files generated)\n");
        } else {
            for path in &self.generated_files {
                output.push_str(&format!("- {}\n", path.display()));
            }
        }
        output.push('\n');

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

        output.push_str("─── Error Information ───\n");
        if let Some(error) = &self.error {
            output.push_str(&format!("Error: {}\n", error));
        } else {
            output.push_str("(no error)\n");
        }

        output.push_str("\n═══════════════════════════════════════════════════════════\n");
        output.push_str("End of Diagnostic Report\n");
        output.push_str("═══════════════════════════════════════════════════════════\n");

        output
    }
}

/// Test harness for artifact generation tests.
///
/// This harness uses the actual TUI code path (BackendEffectHandler + Effects)
/// to ensure tests exercise the real production code.
pub struct TestHarness {
    pub backend: BackendConfiguration,
    pub make: MakeConfiguration,
    pub temp_dir: TempDir,
}

impl TestHarness {
    /// Load an example scenario from the examples directory.
    ///
    /// # Arguments
    /// * `name` - Path relative to examples/ directory (e.g., "scenarios/single-artifact-with-prompts")
    ///
    /// # Returns
    /// A TestHarness with loaded configuration
    pub fn load_example(name: &str) -> Result<Self> {
        let project_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let example_dir = project_root.join("examples").join(name);

        let backend = BackendConfiguration::read_backend_config(&example_dir.join("backend.toml"))
            .with_context(|| format!("Failed to read backend.toml for {}", name))?;

        let make_path = build_make_from_flake(&example_dir)
            .with_context(|| format!("Failed to build make from flake for {}", name))?;
        let make = MakeConfiguration::read_make_config(&make_path)
            .with_context(|| format!("Failed to read make config for {}", name))?;

        let temp_dir = TempDir::new().context("Failed to create temp directory")?;

        let storage_path = temp_dir.path().join("storage");
        std::fs::create_dir_all(&storage_path)
            .context("Failed to create storage directory")?;
        unsafe {
            std::env::set_var("ARTIFACTS_TEST_OUTPUT_DIR", &storage_path);
        }

        Ok(Self {
            backend,
            make,
            temp_dir,
        })
    }

    /// Find the first artifact for a given target (machine or user).
    pub fn find_artifact(
        &self,
        target: &str,
        artifact_name: Option<&str>,
    ) -> Option<(String, ArtifactDef)> {
        if let Some(artifacts) = self.make.nixos_map.get(target) {
            if let Some(name) = artifact_name {
                if let Some(def) = artifacts.get(name) {
                    return Some((name.to_string(), def.clone()));
                }
            } else if let Some((name, def)) = artifacts.iter().next() {
                return Some((name.clone(), def.clone()));
            }
        }

        if let Some(artifacts) = self.make.home_map.get(target) {
            if let Some(name) = artifact_name {
                if let Some(def) = artifacts.get(name) {
                    return Some((name.to_string(), def.clone()));
                }
            } else if let Some((name, def)) = artifacts.iter().next() {
                return Some((name.clone(), def.clone()));
            }
        }

        None
    }

    /// Build a minimal Model with one artifact entry.
    pub fn build_model(
        &self,
        _target: &str,
        artifact: &ArtifactDef,
        target_type: TargetType,
    ) -> Model {
        Model {
            entries: vec![ListEntry::Single(ArtifactEntry {
                target_type,
                artifact: artifact.clone(),
                status: ArtifactStatus::Pending,
                step_logs: StepLogs::default(),
            })],
            ..Default::default()
        }
    }

    /// Create a BackendEffectHandler for this harness.
    pub fn create_handler(&self) -> BackendEffectHandler {
        BackendEffectHandler::new(self.backend.clone(), self.make.clone())
    }

    /// Generate an artifact using the full TUI effect pipeline.
    ///
    /// This method executes the complete pipeline:
    /// 1. CheckSerialization (if artifact exists)
    /// 2. RunGenerator (if needed)
    /// 3. Serialize
    ///
    /// # Arguments
    /// * `target` - Machine or user name
    /// * `artifact` - The artifact definition
    /// * `target_type` - Target type (NixOS or HomeManager)
    /// * `prompts` - Prompt values for the generator
    ///
    /// # Returns
    /// TestArtifactResult with generation outcome
    #[allow(clippy::too_many_lines)]
    pub fn generate_artifact(
        &self,
        target: &str,
        artifact: &ArtifactDef,
        target_type: TargetType,
        prompts: &BTreeMap<String, String>,
    ) -> Result<TestArtifactResult> {
        let model = self.build_model(target, artifact, target_type.clone());
        let mut handler = self.create_handler();

        let check_effect = Effect::CheckSerialization {
            artifact_index: 0,
            artifact_name: artifact.name.clone(),
            target_type: target_type.clone(),
        };

        let messages = handler.execute(check_effect, &model)?;

        let _needs_generation = messages.iter().any(|msg| {
            matches!(
                msg,
                Message::CheckSerializationResult {
                    status: ArtifactStatus::NeedsGeneration,
                    ..
                }
            )
        });

        let prompts_map: HashMap<String, String> = prompts.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

        let gen_effect = Effect::RunGenerator {
            artifact_index: 0,
            artifact_name: artifact.name.clone(),
            target_type: target_type.clone(),
            prompts: prompts_map.clone(),
        };

        let gen_messages = handler.execute(gen_effect, &model)?;

        for msg in &gen_messages {
            if let Message::GeneratorFinished { result, .. } = msg {
                if let Err(e) = result {
                    return Ok(TestArtifactResult {
                        target: target.to_string(),
                        artifact_name: artifact.name.clone(),
                        success: false,
                        error: Some(e.clone()),
                        generated_file_contents: BTreeMap::new(),
                    });
                }
            }
        }

        // Collect generated files BEFORE serialize clears the temp directory
        let mut generated_file_contents = BTreeMap::new();
        if let Some(out_dir) = &handler.current_out_dir {
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
        }

        let serialize_effect = Effect::Serialize {
            artifact_index: 0,
            artifact_name: artifact.name.clone(),
            target_type,
        };

        let serialize_messages = handler.execute(serialize_effect, &model)?;

        for msg in &serialize_messages {
            if let Message::SerializeFinished { result, .. } = msg {
                if let Err(e) = result {
                    return Ok(TestArtifactResult {
                        target: target.to_string(),
                        artifact_name: artifact.name.clone(),
                        success: false,
                        error: Some(e.clone()),
                        generated_file_contents,
                    });
                }
            }
        }

        Ok(TestArtifactResult {
            target: target.to_string(),
            artifact_name: artifact.name.clone(),
            success: true,
            error: None,
            generated_file_contents,
        })
    }

    /// Generate with full diagnostic capture for debugging.
    ///
    /// Similar to `generate_artifact` but also returns diagnostic information.
    #[allow(clippy::too_many_lines)]
    pub fn generate_artifact_with_diagnostics(
        &self,
        target: &str,
        artifact: &ArtifactDef,
        target_type: TargetType,
        prompts: &BTreeMap<String, String>,
    ) -> Result<(TestArtifactResult, DiagnosticInfo)> {
        let mut diagnostics = DiagnosticInfo::new(artifact.name.clone(), target.to_string());

        match std::fs::read_to_string(self.backend.base_path.join("backend.toml")) {
            Ok(config) => diagnostics.backend_config = config,
            Err(e) => diagnostics.backend_config = format!("(failed to read: {})", e),
        }

        diagnostics.make_config = format!(
            "make_base: {}\nnixos_map keys: {:?}\nhome_map keys: {:?}",
            self.make.make_base.display(),
            self.make.nixos_map.keys().collect::<Vec<_>>(),
            self.make.home_map.keys().collect::<Vec<_>>()
        );

        for (key, value) in std::env::vars() {
            if key.starts_with("ARTIFACTS_") || key.starts_with("CARGO_") {
                diagnostics.environment_vars.insert(key, value);
            }
        }

        for (prompt_name, _value) in prompts {
            diagnostics
                .temp_prompt_contents
                .insert(prompt_name.clone(), "[REDACTED]".to_string());
        }

        let result = self.generate_artifact(target, artifact, target_type.clone(), prompts)?;

        if let Some(ref error) = result.error {
            diagnostics.error = Some(error.clone());
        }

        if let Some(out_dir) = std::env::var("ARTIFACTS_TEST_OUTPUT_DIR")
            .ok()
            .map(|p| std::path::PathBuf::from(p))
        {
            for file_name in artifact.files.keys() {
                let file_path = out_dir.join(file_name);
                if file_path.exists() {
                    diagnostics.generated_files.push(file_path);
                }
            }
        }

        Ok((result, diagnostics))
    }
}

impl Drop for TestHarness {
    fn drop(&mut self) {
        unsafe {
            std::env::remove_var("ARTIFACTS_TEST_OUTPUT_DIR");
        }
    }
}

/// RAII guard to ensure environment cleanup even on panic.
pub struct CleanupGuard;

impl Drop for CleanupGuard {
    fn drop(&mut self) {
        unsafe {
            std::env::remove_var("ARTIFACTS_TEST_OUTPUT_DIR");
        }
    }
}

/// Set up test storage directory with ARTIFACTS_TEST_OUTPUT_DIR.
///
/// Returns a TempDir that should be kept alive for the duration of the test.
pub fn setup_test_storage() -> Result<(TempDir, PathBuf)> {
    let temp_dir = TempDir::new()?;
    let storage_path = temp_dir.path().join("storage");
    std::fs::create_dir_all(&storage_path)?;

    unsafe {
        std::env::set_var("ARTIFACTS_TEST_OUTPUT_DIR", &storage_path);
    }

    Ok((temp_dir, storage_path))
}

/// Clean up environment after test.
pub fn cleanup_test_storage() {
    unsafe {
        std::env::remove_var("ARTIFACTS_TEST_OUTPUT_DIR");
    }
}

/// Get the project root directory.
pub fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Dump diagnostic information to a file for test failure investigation.
pub fn dump_test_diagnostics(diag: &DiagnosticInfo, output_path: &std::path::Path) -> Result<()> {
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }

    let formatted = diag.format();
    std::fs::write(output_path, formatted)
        .with_context(|| format!("Failed to write diagnostic to: {}", output_path.display()))?;

    Ok(())
}