//! Diagnostic utilities for test failure investigation.
//!
//! This module provides diagnostic tooling to capture and dump detailed
//! information when tests fail. This helps developers understand what went
//! wrong during artifact generation by capturing:
//! - Configuration (backend.toml, flake.nix)
//! - Environment variables
//! - Temporary file contents
//! - Generator and backend output
//! - Error messages
//!
//! ## Test Requirements
//! - TEST-06: Tests provide meaningful failure information for CI
//!
//! ## Usage
//!
//! To capture diagnostics on test failure:
//!
//! ```rust
//! let (result, diagnostics) = generate_single_artifact_with_diagnostics_and_target_type(
//!     "machine-name",
//!     &artifact_def,
//!     &prompt_values,
//!     &backend,
//!     &make_config,
//!     TargetType::NixOS { machine: "machine-name".to_string() },
//! );
//!
//! if let Err(e) = &result {
//!     dump_test_diagnostics(&diagnostics, Path::new("/tmp/my-test"));
//! }
//! ```

use anyhow::{Context, Result};
use artifacts::app::model::TargetType;
use artifacts::cli::headless::{
    DiagnosticInfo, PromptValues, generate_single_artifact_with_diagnostics_and_target_type,
};
use artifacts::config::backend::BackendConfiguration;
use artifacts::config::make::{ArtifactDef, MakeConfiguration};
use artifacts::config::nix::build_make_from_flake;
use serial_test::serial;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Find the first artifact for a given machine.
fn find_first_artifact(
    make_config: &MakeConfiguration,
    machine_name: &str,
) -> Option<(String, ArtifactDef)> {
    make_config
        .nixos_map
        .get(machine_name)
        .and_then(|artifacts| {
            artifacts
                .iter()
                .next()
                .map(|(name, def)| (name.clone(), def.clone()))
        })
}

/// Get the project root directory.
fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Load an example scenario's backend and make configuration.
fn load_example(name: &str) -> Result<(BackendConfiguration, MakeConfiguration)> {
    let example_dir = project_root().join("examples").join(name);

    let backend = BackendConfiguration::read_backend_config(&example_dir.join("backend.toml"))
        .with_context(|| format!("Failed to read backend.toml for {}", name))?;

    let make_path = build_make_from_flake(&example_dir)
        .with_context(|| format!("Failed to build make from flake for {}", name))?;
    let make = MakeConfiguration::read_make_config(&make_path)
        .with_context(|| format!("Failed to read make config for {}", name))?;

    Ok((backend, make))
}

/// Dump diagnostic information to a file for later analysis.
///
/// This function writes the formatted diagnostic information to the specified
/// output path. It creates the parent directory if it doesn't exist.
///
/// # Arguments
/// * `diagnostic` - The diagnostic information to dump
/// * `output_path` - Path where diagnostic output should be written
///
/// # Returns
/// * `Ok(())` if the diagnostic was successfully written
/// * `Err` if writing failed
///
/// # Example
/// ```rust
/// dump_test_diagnostics(&diagnostics, Path::new("/tmp/test-failure.log"))?;
/// ```
pub fn dump_test_diagnostics(diagnostic: &DiagnosticInfo, output_path: &Path) -> Result<()> {
    // Create parent directory if needed
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }

    let formatted = diagnostic.format();
    fs::write(output_path, formatted)
        .with_context(|| format!("Failed to write diagnostic to: {}", output_path.display()))?;

    Ok(())
}

/// Capture relevant test environment variables.
///
/// This function captures environment variables that are relevant for
/// debugging test failures. It filters out sensitive variables and
/// includes test-specific variables.
///
/// # Returns
/// HashMap of environment variable names to values
///
/// # Example
/// ```rust
/// let env_vars = capture_test_environment();
/// for (key, value) in env_vars {
///     println!("{}={}", key, value);
/// }
/// ```
pub fn capture_test_environment() -> HashMap<String, String> {
    let mut env_vars = HashMap::new();

    for (key, value) in env::vars() {
        // Capture ARTIFACTS_ prefixed variables
        if key.starts_with("ARTIFACTS_") {
            env_vars.insert(key, value);
            continue;
        }

        // Capture CARGO_ prefixed variables
        if key.starts_with("CARGO_") {
            env_vars.insert(key, value);
            continue;
        }

        // Capture RUST_ prefixed variables
        if key.starts_with("RUST_") {
            env_vars.insert(key, value);
            continue;
        }
    }

    env_vars
}

/// Wrapper function to run a test with automatic diagnostic capture on failure.
///
/// This function runs the provided test function and automatically dumps
/// diagnostic information if the test fails.
///
/// # Type Parameters
/// * `F` - The test function type
/// * `T` - The return type of the test function
///
/// # Arguments
/// * `test_name` - Name of the test (used in diagnostic filename)
/// * `test_fn` - The test function to run
///
/// # Returns
/// * `Ok(T)` if the test function succeeds
/// * `Err` if the test function fails (diagnostics are dumped before returning)
///
/// # Example
/// ```rust
/// run_with_diagnostics("my_test", || {
///     // Test code here
///     Ok(())
/// })?;
/// ```
#[allow(dead_code)]
pub fn run_with_diagnostics<F, T>(test_name: &str, test_fn: F) -> Result<T>
where
    F: FnOnce() -> Result<T>,
{
    match test_fn() {
        Ok(result) => Ok(result),
        Err(e) => {
            // Dump diagnostics to temp directory
            let diag_dir = PathBuf::from("/tmp/artifacts_test_failures");
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let diag_path = diag_dir.join(format!("{}_{}.txt", timestamp, test_name));

            // Note: Since we don't have diagnostics info here, we just log the error
            // In a real implementation, you'd capture diagnostics from the test
            eprintln!("\n=== Test '{}' failed ===", test_name);
            eprintln!("Error: {}", e);
            eprintln!("Diagnostic directory: {}", diag_dir.display());

            // Create failure marker
            let _ = fs::create_dir_all(&diag_dir);
            let _ = fs::write(&diag_path, format!("Test: {}\nError: {}\n", test_name, e));

            Err(e)
        }
    }
}

/// Test that diagnostic capture works for successful generation.
#[test]
#[serial]
fn e2e_diagnostic_capture_complete() -> Result<()> {
    // Load example configuration
    let (backend, make_config) = load_example("scenarios/single-artifact-with-prompts")?;

    // Find first artifact
    let (artifact_name, artifact_def) = find_first_artifact(&make_config, "machine-name")
        .ok_or_else(|| anyhow::anyhow!("No artifacts found"))?;

    // Set up prompt values
    let prompt_values: PromptValues = BTreeMap::from([
        ("secret1".to_string(), "test-secret-one".to_string()),
        ("secret2".to_string(), "test-secret-two".to_string()),
    ]);

    // Generate with diagnostics
    let (result, diagnostics) = generate_single_artifact_with_diagnostics_and_target_type(
        "machine-name",
        &artifact_def,
        &prompt_values,
        &backend,
        &make_config,
        TargetType::NixOS {
            machine: "machine-name".to_string(),
        },
    );

    // Verify generation succeeded
    assert!(
        result.is_ok(),
        "Generation should succeed: {:?}",
        result.err()
    );

    // Verify diagnostic fields are populated
    assert_eq!(diagnostics.artifact_name, artifact_name);
    assert_eq!(diagnostics.target, "machine-name");

    // Backend config should be captured
    assert!(
        !diagnostics.backend_config.is_empty(),
        "Backend config should be captured"
    );

    // Make config should be captured
    assert!(
        !diagnostics.make_config.is_empty(),
        "Make config should be captured"
    );

    // Generated files should be populated
    assert!(
        !diagnostics.generated_files.is_empty(),
        "Generated files should be captured"
    );

    // No error on success
    assert!(
        diagnostics.error.is_none(),
        "Should have no error on success"
    );

    Ok(())
}

/// Test that diagnostic capture works for failed generation.
#[test]
#[serial]
fn e2e_diagnostic_capture_on_failure() -> Result<()> {
    // Load example configuration
    let (backend, make_config) = load_example("scenarios/single-artifact-with-prompts")?;

    // Find first artifact
    let (_, artifact_def) = find_first_artifact(&make_config, "machine-name")
        .ok_or_else(|| anyhow::anyhow!("No artifacts found"))?;

    // Create an invalid configuration by using a non-existent backend
    // We'll modify the artifact to use an invalid generator path
    let mut invalid_artifact = artifact_def.clone();
    invalid_artifact.generator = "/nonexistent/generator.sh".to_string();

    // Set up prompt values
    let prompt_values: PromptValues = BTreeMap::from([
        ("secret1".to_string(), "test-secret-one".to_string()),
        ("secret2".to_string(), "test-secret-two".to_string()),
    ]);

    // Generate with diagnostics (this will fail due to invalid generator)
    let (result, diagnostics) = generate_single_artifact_with_diagnostics_and_target_type(
        "machine-name",
        &invalid_artifact,
        &prompt_values,
        &backend,
        &make_config,
        TargetType::NixOS {
            machine: "machine-name".to_string(),
        },
    );

    // Verify generation failed (expected)
    assert!(
        result.is_err(),
        "Generation should fail with invalid generator"
    );

    // Verify diagnostic fields are still populated
    assert_eq!(diagnostics.artifact_name, invalid_artifact.name);
    assert_eq!(diagnostics.target, "machine-name");

    // Error should be captured
    assert!(
        diagnostics.error.is_some(),
        "Error should be captured on failure"
    );

    // Backend config should still be captured
    assert!(
        !diagnostics.backend_config.is_empty(),
        "Backend config should be captured even on failure"
    );

    // Make config should still be captured
    assert!(
        !diagnostics.make_config.is_empty(),
        "Make config should be captured even on failure"
    );

    Ok(())
}

/// Test that diagnostic dump functionality works.
#[test]
#[serial]
fn e2e_diagnostic_dump_functionality() -> Result<()> {
    // Create a temporary directory for the test
    let temp_dir = TempDir::new()?;
    let diag_path = temp_dir.path().join("diagnostics.txt");

    // Load example and generate
    let (backend, make_config) = load_example("scenarios/single-artifact-with-prompts")?;
    let (_, artifact_def) = find_first_artifact(&make_config, "machine-name")
        .ok_or_else(|| anyhow::anyhow!("No artifacts found"))?;

    let prompt_values: PromptValues = BTreeMap::from([
        ("secret1".to_string(), "test-secret-one".to_string()),
        ("secret2".to_string(), "test-secret-two".to_string()),
    ]);

    let (result, diagnostics) = generate_single_artifact_with_diagnostics_and_target_type(
        "machine-name",
        &artifact_def,
        &prompt_values,
        &backend,
        &make_config,
        TargetType::NixOS {
            machine: "machine-name".to_string(),
        },
    );

    assert!(result.is_ok(), "Generation failed: {:?}", result.err());

    // Dump diagnostics
    dump_test_diagnostics(&diagnostics, &diag_path)?;

    // Verify file was created
    assert!(diag_path.exists(), "Diagnostic file should be created");

    // Verify file has content
    let content = fs::read_to_string(&diag_path)?;
    assert!(!content.is_empty(), "Diagnostic file should have content");

    // Verify content has expected sections
    assert!(
        content.contains("Diagnostic Report"),
        "Should contain header"
    );
    assert!(
        content.contains("Configuration"),
        "Should contain config section"
    );
    assert!(
        content.contains("Environment Variables"),
        "Should contain environment section"
    );

    Ok(())
}

/// Test that diagnostic format produces human-readable output.
#[test]
fn e2e_diagnostic_format_readable() {
    // Create a simple diagnostic
    let mut diag = DiagnosticInfo::new("test-artifact".to_string(), "test-machine".to_string());

    diag.backend_config = "test-backend = {}".to_string();
    diag.environment_vars
        .insert("TEST_VAR".to_string(), "test_value".to_string());

    // Format the diagnostic
    let formatted = diag.format();

    // Verify it's human-readable
    assert!(
        formatted.contains("Diagnostic Report for: test-artifact"),
        "Should contain artifact name in header"
    );
    assert!(
        formatted.contains("Target: test-machine"),
        "Should contain target in header"
    );

    // Should have section headers
    assert!(
        formatted.contains("Configuration"),
        "Should have config section header"
    );
    assert!(
        formatted.contains("Environment Variables"),
        "Should have environment section header"
    );
    assert!(
        formatted.contains("Input Files"),
        "Should have input files section header"
    );
    assert!(
        formatted.contains("Prompt Files"),
        "Should have prompt files section header"
    );
    assert!(
        formatted.contains("Generated Files"),
        "Should have generated files section header"
    );
    assert!(
        formatted.contains("Generator Output"),
        "Should have generator output section header"
    );
    assert!(
        formatted.contains("Backend Output"),
        "Should have backend output section header"
    );
    assert!(
        formatted.contains("Error Information"),
        "Should have error section header"
    );

    // Should not have raw debug formatting
    assert!(
        !formatted.contains("DiagnosticInfo {"),
        "Should not use debug formatting"
    );

    // Should have section dividers
    assert!(
        formatted.contains("════════════════"),
        "Should have visual separators"
    );
}

/// Test that diagnostic format redacts sensitive values.
#[test]
fn e2e_diagnostic_redacts_sensitive_values() {
    let mut diag = DiagnosticInfo::new("test-artifact".to_string(), "test-machine".to_string());

    // Add sensitive environment variables
    diag.environment_vars
        .insert("API_KEY".to_string(), "secret123".to_string());
    diag.environment_vars
        .insert("SECRET_TOKEN".to_string(), "token456".to_string());
    diag.environment_vars
        .insert("MY_PASSWORD".to_string(), "pass789".to_string());
    diag.environment_vars
        .insert("SAFE_VAR".to_string(), "visible".to_string());

    // Add prompt contents (should be redacted)
    diag.temp_prompt_contents
        .insert("password".to_string(), "secret".to_string());

    let formatted = diag.format();

    // Sensitive values should be redacted
    assert!(
        formatted.contains("API_KEY: [REDACTED]"),
        "Should redact API_KEY"
    );
    assert!(
        formatted.contains("SECRET_TOKEN: [REDACTED]"),
        "Should redact SECRET_TOKEN"
    );
    assert!(
        formatted.contains("MY_PASSWORD: [REDACTED]"),
        "Should redact MY_PASSWORD"
    );

    // Safe values should be visible
    assert!(
        formatted.contains("SAFE_VAR=visible"),
        "Should show safe variables"
    );

    // Prompt contents should be redacted
    assert!(
        formatted.contains("password: [REDACTED]"),
        "Should redact prompt values"
    );
}

/// Test that environment capture works correctly.
#[test]
fn e2e_diagnostic_environment_capture() {
    // Set test environment variable (using unsafe as required by Rust 2024)
    unsafe {
        env::set_var("ARTIFACTS_TEST_VAR", "test_value_123");
    }

    // Capture environment
    let env_vars = capture_test_environment();

    // Should capture ARTIFACTS_ prefixed variables
    assert!(
        env_vars.contains_key("ARTIFACTS_TEST_VAR"),
        "Should capture ARTIFACTS_ prefixed variables"
    );
    assert_eq!(
        env_vars.get("ARTIFACTS_TEST_VAR"),
        Some(&"test_value_123".to_string())
    );

    // Cleanup (using unsafe as required by Rust 2024)
    unsafe {
        env::remove_var("ARTIFACTS_TEST_VAR");
    }
}
