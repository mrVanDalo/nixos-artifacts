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
//! let harness = TestHarness::load_example("scenarios/single-artifact-with-prompts")?;
//! let (result, diagnostics) = harness.generate_artifact_with_diagnostics(
//!     "machine-name",
//!     &artifact_def,
//!     TargetType::NixOS { machine: "machine-name".to_string() },
//!     &prompts,
//! )?;
//!
//! if !result.success {
//!     dump_test_diagnostics(&diagnostics, Path::new("/tmp/my-test"));
//! }
//! ```

use crate::common::{DiagnosticInfo, TestHarness, dump_test_diagnostics};
use anyhow::Result;
use artifacts::app::model::TargetType;
use serial_test::serial;
use std::collections::HashMap;
use std::env;
use std::fs;
use tempfile::TempDir;

#[test]
#[serial]
fn e2e_diagnostic_capture_complete() -> Result<()> {
    let harness = TestHarness::load_example("scenarios/single-artifact-with-prompts")?;

    let (artifact_name, artifact_def) = harness
        .find_artifact("machine-name", None)
        .ok_or_else(|| anyhow::anyhow!("No artifacts found"))?;

    let prompt_values: std::collections::BTreeMap<String, String> =
        std::collections::BTreeMap::from([
            ("secret1".to_string(), "test-secret-one".to_string()),
            ("secret2".to_string(), "test-secret-two".to_string()),
        ]);

    let (result, diagnostics) = harness.generate_artifact_with_diagnostics(
        "machine-name",
        &artifact_def,
        TargetType::NixOS {
            machine: "machine-name".to_string(),
        },
        &prompt_values,
    )?;

    assert!(
        result.success,
        "Generation should succeed: {:?}",
        result.error
    );

    assert_eq!(diagnostics.artifact_name, artifact_name);
    assert_eq!(diagnostics.target, "machine-name");

    assert!(
        !diagnostics.backend_config.is_empty(),
        "Backend config should be captured"
    );

    assert!(
        !diagnostics.make_config.is_empty(),
        "Make config should be captured"
    );

    assert!(
        !diagnostics.generated_files.is_empty() || result.success,
        "Generated files should be captured on success"
    );

    assert!(
        diagnostics.error.is_none(),
        "Should have no error on success"
    );

    Ok(())
}

#[test]
#[serial]
fn e2e_diagnostic_capture_on_failure() -> Result<()> {
    let harness = TestHarness::load_example("scenarios/single-artifact-with-prompts")?;

    let (_, artifact_def) = harness
        .find_artifact("machine-name", None)
        .ok_or_else(|| anyhow::anyhow!("No artifacts found"))?;

    let mut invalid_artifact = artifact_def.clone();
    invalid_artifact.generator = "/nonexistent/generator.sh".to_string();

    let prompt_values: std::collections::BTreeMap<String, String> =
        std::collections::BTreeMap::from([
            ("secret1".to_string(), "test-secret-one".to_string()),
            ("secret2".to_string(), "test-secret-two".to_string()),
        ]);

    let (result, diagnostics) = harness.generate_artifact_with_diagnostics(
        "machine-name",
        &invalid_artifact,
        TargetType::NixOS {
            machine: "machine-name".to_string(),
        },
        &prompt_values,
    )?;

    assert!(
        !result.success,
        "Generation should fail with invalid generator"
    );

    assert_eq!(diagnostics.artifact_name, invalid_artifact.name);
    assert_eq!(diagnostics.target, "machine-name");

    assert!(
        diagnostics.error.is_some(),
        "Error should be captured on failure"
    );

    assert!(
        !diagnostics.backend_config.is_empty(),
        "Backend config should be captured even on failure"
    );

    assert!(
        !diagnostics.make_config.is_empty(),
        "Make config should be captured even on failure"
    );

    Ok(())
}

#[test]
#[serial]
fn e2e_diagnostic_dump_functionality() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let diag_path = temp_dir.path().join("diagnostics.txt");

    let harness = TestHarness::load_example("scenarios/single-artifact-with-prompts")?;
    let (_, artifact_def) = harness
        .find_artifact("machine-name", None)
        .ok_or_else(|| anyhow::anyhow!("No artifacts found"))?;

    let prompt_values: std::collections::BTreeMap<String, String> =
        std::collections::BTreeMap::from([
            ("secret1".to_string(), "test-secret-one".to_string()),
            ("secret2".to_string(), "test-secret-two".to_string()),
        ]);

    let (result, diagnostics) = harness.generate_artifact_with_diagnostics(
        "machine-name",
        &artifact_def,
        TargetType::NixOS {
            machine: "machine-name".to_string(),
        },
        &prompt_values,
    )?;

    assert!(result.success, "Generation failed: {:?}", result.error);

    dump_test_diagnostics(&diagnostics, &diag_path)?;

    assert!(diag_path.exists(), "Diagnostic file should be created");

    let content = fs::read_to_string(&diag_path)?;
    assert!(!content.is_empty(), "Diagnostic file should have content");

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

#[test]
fn e2e_diagnostic_format_readable() {
    let mut diag = DiagnosticInfo::new("test-artifact".to_string(), "test-machine".to_string());

    diag.backend_config = "test-backend = {}".to_string();
    diag.environment_vars
        .insert("TEST_VAR".to_string(), "test_value".to_string());

    let formatted = diag.format();

    assert!(
        formatted.contains("Diagnostic Report for: test-artifact"),
        "Should contain artifact name in header"
    );
    assert!(
        formatted.contains("Target: test-machine"),
        "Should contain target in header"
    );

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

    assert!(
        !formatted.contains("DiagnosticInfo {"),
        "Should not use debug formatting"
    );

    assert!(
        formatted.contains("════════════════"),
        "Should have visual separators"
    );
}

#[test]
fn e2e_diagnostic_redacts_sensitive_values() {
    let mut diag = DiagnosticInfo::new("test-artifact".to_string(), "test-machine".to_string());

    diag.environment_vars
        .insert("API_KEY".to_string(), "secret123".to_string());
    diag.environment_vars
        .insert("SECRET_TOKEN".to_string(), "token456".to_string());
    diag.environment_vars
        .insert("MY_PASSWORD".to_string(), "pass789".to_string());
    diag.environment_vars
        .insert("SAFE_VAR".to_string(), "visible".to_string());

    diag.temp_prompt_contents
        .insert("password".to_string(), "secret".to_string());

    let formatted = diag.format();

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

    assert!(
        formatted.contains("SAFE_VAR=visible"),
        "Should show safe variables"
    );

    assert!(
        formatted.contains("password: [REDACTED]"),
        "Should redact prompt values"
    );
}

#[test]
fn e2e_diagnostic_environment_capture() {
    unsafe {
        env::set_var("ARTIFACTS_TEST_VAR", "test_value_123");
    }

    let env_vars = capture_test_environment();

    assert!(
        env_vars.contains_key("ARTIFACTS_TEST_VAR"),
        "Should capture ARTIFACTS_ prefixed variables"
    );
    assert_eq!(
        env_vars.get("ARTIFACTS_TEST_VAR"),
        Some(&"test_value_123".to_string())
    );

    unsafe {
        env::remove_var("ARTIFACTS_TEST_VAR");
    }
}

fn capture_test_environment() -> HashMap<String, String> {
    let mut env_vars = HashMap::new();

    for (key, value) in env::vars() {
        if key.starts_with("ARTIFACTS_") {
            env_vars.insert(key, value);
            continue;
        }

        if key.starts_with("CARGO_") {
            env_vars.insert(key, value);
            continue;
        }

        if key.starts_with("RUST_") {
            env_vars.insert(key, value);
            continue;
        }
    }

    env_vars
}

struct CleanupGuard;

impl Drop for CleanupGuard {
    fn drop(&mut self) {
        unsafe {
            std::env::remove_var("ARTIFACTS_TEST_OUTPUT_DIR");
        }
    }
}
