# Testing Guide for Artifacts CLI

This document describes the testing infrastructure and diagnostic capabilities
for the artifacts CLI project.

## Overview

The artifacts CLI uses a comprehensive testing strategy:

- **Unit tests** - Testing individual functions and modules
- **Integration tests** - Testing backend operations and CLI commands
- **End-to-end (e2e) tests** - Testing the complete artifact generation flow
- **Async tests** - Testing the TUI runtime and event handling
- **Snapshot tests** - Testing CLI output formatting

## Test Structure

```
tests/
├── async_tests/         # Async runtime tests
├── backend/            # Backend operation tests
├── cli/                # CLI command tests (insta-cmd)
├── e2e/                # End-to-end integration tests
│   ├── mod.rs          # Core e2e tests
│   ├── backend_verify.rs   # Backend storage verification
│   ├── diagnostics.rs  # Diagnostic tooling
│   ├── edge_cases.rs   # Edge case and error handling
│   └── shared_artifact.rs  # Shared artifact tests
└── tests.rs            # Test entry point
```

## Running Tests

### Run all tests

```bash
cargo test --test tests -- --test-threads=1
```

### Run specific test categories

```bash
# E2E tests only
cargo test --test tests e2e -- --test-threads=1

# Backend tests
cargo test --test tests backend

# CLI tests
cargo test --test tests cli

# Async tests
cargo test --test tests async_tests
```

### Run a specific test

```bash
cargo test --test tests e2e_single_artifact_is_created -- --test-threads=1
```

### Important flags

- `--test-threads=1`: Required for e2e tests to prevent shared state conflicts
- `--no-run`: Compile tests without running (useful for checking compilation)

## Diagnostic System

The diagnostic system captures detailed information during test execution to
help debug failures.

### Diagnostic Information Captured

When tests fail, the diagnostic system captures:

1. **Configuration**
   - Backend configuration (backend.toml contents)
   - Make configuration (nixos_map and home_map keys)

2. **Environment Variables**
   - ARTIFACTS_ prefixed variables
   - CARGO_ prefixed variables
   - RUST_ prefixed variables

3. **Temporary Files**
   - Input file contents
   - Prompt file names (values redacted for security)

4. **Generated Files**
   - Paths to generated files
   - File contents

5. **Error Information**
   - Error messages
   - Generator stderr (if captured)
   - Backend stderr (if captured)

### Security Considerations

The diagnostic system automatically redacts:

- Prompt values (replaced with `[REDACTED]`)
- Environment variables containing sensitive keywords:
  - Variables with "SECRET" in the name
  - Variables with "PASSWORD" in the name
  - Variables with "TOKEN" in the name
  - Variables with "KEY" in the name

### Using Diagnostics in Tests

To capture diagnostics on test failure:

```rust
use artifacts::cli::headless::generate_single_artifact_with_diagnostics;
use crate::e2e::dump_test_diagnostics;

#[test]
fn my_test() -> Result<()> {
    let (result, diagnostics) = generate_single_artifact_with_diagnostics(
        "machine-name",
        &artifact_def,
        &prompt_values,
        &backend,
        &make_config,
    );

    match result {
        Ok(r) => r,
        Err(e) => {
            // Dump diagnostics on failure
            let diag_path = PathBuf::from("/tmp/diagnostics.txt");
            dump_test_diagnostics(&diagnostics, &diag_path)?;
            return Err(e);
        }
    }

    // Test assertions...
    Ok(())
}
```

### Automatic Diagnostic Dump

The `e2e_single_artifact_is_created` test now automatically dumps diagnostics
on failure to `/tmp/artifacts_test_failures/` with a timestamp in the filename.

## Troubleshooting Failed Tests

### Finding Diagnostic Output

When e2e tests fail, check:

```
/tmp/artifacts_test_failures/
├── {timestamp}_{test_name}.txt
└── ...
```

### Reading Diagnostic Output

Diagnostic files are formatted with clear sections:

```
═══════════════════════════════════════════════════════════
Diagnostic Report for: test-artifact
Target: machine-name
═══════════════════════════════════════════════════════════

─── Configuration ───
Backend Config:
[backend contents]

Make Config:
[configuration summary]

─── Environment Variables ───
CARGO_PKG_NAME=artifacts
...

─── Input Files ───
(no input files)

─── Prompt Files ───
(no prompt files - prompt values redacted)

─── Generated Files ───
- /tmp/artifacts-headless-xxx/out/very-simple-secrets

─── Generator Output ───
stdout: (not captured)
stderr: (not captured)

─── Backend Output ───
stdout: (not captured)
stderr: (not captured)

─── Error Information ───
Error: [error message if present]

═══════════════════════════════════════════════════════════
End of Diagnostic Report
═══════════════════════════════════════════════════════════
```

### Common Test Failures

**1. Generator failed with non-zero exit status**

- Check the artifact's generator script
- Verify prompts are being passed correctly
- Look at the prompt file contents in diagnostics

**2. Backend configuration not found**

- Verify backend.toml exists in the scenario directory
- Check the backend.toml syntax

**3. Serialization failed**

- Check the serialize script exists and is executable
- Verify the backend configuration is correct

### Debug Mode

Enable debug logging for more verbose output:

```bash
RUST_LOG=debug cargo test --test tests e2e -- --test-threads=1
```

## Test Requirements

All e2e tests require:

- Nix installation with flake support
- Scenarios in `examples/scenarios/` directory
- serial_test for test isolation (`#[serial]`)
- Tests run single-threaded (`--test-threads=1`)

## CI Testing

Tests are configured to run in CI with:

- Single-threaded execution (`--test-threads=1`)
- Serial test isolation (`#[serial]` attribute)
- Nix environment with flake support
- Automatic diagnostic dump on failure

## Adding New Tests

### E2E Test Template

```rust
#[test]
#[serial]
fn e2e_my_new_test() -> Result<()> {
    // Load example configuration
    let (backend, make_config) = load_example("scenarios/my-scenario")?;

    // Get artifact
    let (_, artifact_def) = find_first_artifact(&make_config, "machine-name")
        .ok_or_else(|| anyhow::anyhow!("No artifacts found"))?;

    // Set up prompts
    let prompt_values: PromptValues = BTreeMap::from([
        ("prompt1".to_string(), "value1".to_string()),
    ]);

    // Generate with diagnostics
    let (result, diagnostics) = generate_single_artifact_with_diagnostics(
        "machine-name",
        &artifact_def,
        &prompt_values,
        &backend,
        &make_config,
    );

    // Handle with diagnostic dump on failure
    let result = match result {
        Ok(r) => r,
        Err(e) => {
            let diag_dir = PathBuf::from("/tmp/artifacts_test_failures");
            let _ = fs::create_dir_all(&diag_dir);
            let diag_path = diag_dir.join("my_new_test.txt");
            let _ = dump_test_diagnostics(&diagnostics, &diag_path);
            return Err(e);
        }
    };

    // Assert success
    assert!(result.success, "Generation should succeed");

    Ok(())
}
```

## Test Categories

### TEST-01: Programmatic Invocation

Verify the headless API can be called programmatically without TUI.

### TEST-02: Single Artifact Creation

Verify single artifacts can be created with simple configurations.

### TEST-03: Artifact Existence Verification

Verify artifacts exist at expected backend locations.

### TEST-04: Content Verification

Verify artifact content matches expected format.

### TEST-05: Shared Artifacts

Verify shared artifacts work correctly across multiple targets.

### TEST-06: CI Failure Messages

Verify tests provide meaningful failure messages for CI debugging.

## Related Files

- `tests/e2e/diagnostics.rs` - Diagnostic utilities
- `src/cli/headless.rs` - Headless API with diagnostic capture
- `tests/tests.rs` - Test entry point
