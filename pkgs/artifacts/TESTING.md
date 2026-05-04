# Testing Guide for Artifacts CLI

This document describes the testing infrastructure and diagnostic capabilities
for the artifacts CLI project.

## Overview

The artifacts CLI uses a layered testing strategy:

- **Unit tests** — Inline `#[cfg(test)]` modules in `src/` for individual
  functions
- **Integration tests** (`tests/`) — Driven through `tests/tests.rs` which
  aggregates every other module in `tests/`
- **End-to-end (e2e) tests** (`tests/e2e/`) — Drive the real backend pipeline
  (check → generate → serialize) against scenarios in `examples/scenarios/`
- **Async tests** (`tests/async_tests/`) — Tokio runtime, channel, and
  state-machine tests for the TUI background plumbing
- **Snapshot tests** — `insta` (TUI views, config parsing) and `insta-cmd` (CLI
  surface)

## Test Structure

```
tests/
├── async_tests/        # Async runtime / channel / shutdown tests
├── backend/            # Generator + serialization unit-style tests
├── cli/                # CLI integration tests (insta-cmd)
├── common/             # Shared TestHarness + diagnostic helpers
├── config/             # backend.toml + make.json parser tests
├── e2e/                # End-to-end pipeline tests
│   ├── mod.rs              # Core e2e tests
│   ├── backend_verify.rs   # Backend storage verification
│   ├── config_env_tests.rs # $targets / $config env wiring
│   ├── diagnostics.rs      # Diagnostic-system tests
│   ├── edge_cases.rs       # Error scenarios
│   └── shared_artifact.rs  # Shared artifact tests
├── tui/                # View snapshots + interaction tests
├── test_helpers.rs     # Misc helpers
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

- `--test-threads=1`: Required for e2e tests because they share temp output
  state via `ARTIFACTS_TEST_OUTPUT_DIR`
- `--no-run`: Compile tests without running (useful for type-check loops)

## Test Harness

`tests/common/mod.rs` exposes `TestHarness`, the entry point used by every e2e
and most integration tests. It owns the loaded `BackendConfiguration`,
`MakeConfiguration`, and a per-test `TempDir` for backend storage.

```rust
use artifacts::app::model::TargetType;
use std::collections::BTreeMap;
use crate::common::{TestHarness, dump_test_diagnostics};

let harness = TestHarness::load_example("scenarios/single-artifact-with-prompts")?;

let (artifact_name, artifact_def) = harness
    .find_artifact("machine-name", None)
    .ok_or_else(|| anyhow::anyhow!("no artifacts for machine-name"))?;

let target_type = TargetType::NixOS {
    machine: "machine-name".to_string(),
};

let prompts: BTreeMap<String, String> = BTreeMap::from([
    ("secret1".to_string(), "test-secret-one".to_string()),
]);

let result = harness.generate_artifact(
    "machine-name",
    &artifact_def,
    target_type,
    &prompts,
)?;

assert!(result.success, "generation should succeed");
```

`generate_artifact` runs the full backend pipeline directly
(`run_check_serialization` → `run_generator_script` → `run_serialize`) so tests
exercise the production code paths without going through the TUI.

## Diagnostic System

When a generation fails, `generate_artifact_with_diagnostics` returns a
`DiagnosticInfo` alongside the result. Persist it to disk on failure to
investigate later:

```rust
let (result, diagnostics) = harness.generate_artifact_with_diagnostics(
    "machine-name",
    &artifact_def,
    target_type,
    &prompts,
)?;

if !result.success {
    let diag_dir = std::path::PathBuf::from("/tmp/artifacts_test_failures");
    let diag_path = diag_dir.join("my_test.txt");
    dump_test_diagnostics(&diagnostics, &diag_path)?;
    anyhow::bail!("generation failed: see {}", diag_path.display());
}
```

### What gets captured

`DiagnosticInfo` (see `tests/common/mod.rs`) records:

1. **Configuration** — `backend.toml` contents and a summary of
   `MakeConfiguration` (base path + map keys)
2. **Environment variables** — `ARTIFACTS_*` and `CARGO_*` only; other prefixes
   are not collected
3. **Prompt files** — names only; values are stored as `[REDACTED]`
4. **Generated files** — paths under `ARTIFACTS_TEST_OUTPUT_DIR`
5. **Errors** — propagated error message from the failed pipeline step

### Redaction

`DiagnosticInfo::format` redacts any captured environment variable whose key
(uppercased) contains `SECRET`, `PASSWORD`, `TOKEN`, or `KEY`. Prompt values are
never captured in plaintext — only the prompt name is recorded.

### Diagnostic report layout

`format()` produces a report like this:

```
═══════════════════════════════════════════════════════════
Diagnostic Report for: my-artifact
Target: machine-name
═══════════════════════════════════════════════════════════

─── Configuration ───
Backend Config:
[backend.toml contents]

Make Config:
make_base: /…
nixos_map keys: [...]
home_map keys: [...]

─── Environment Variables ───
ARTIFACTS_TEST_OUTPUT_DIR=…
CARGO_PKG_NAME=artifacts
…

─── Input Files ───
(no input files)

─── Prompt Files ───
secret1: [REDACTED]

─── Generated Files ───
- /tmp/…/my-artifact

─── Generator Output ───
stdout: (not captured)
stderr: (not captured)

─── Backend Output ───
stdout: (not captured)
stderr: (not captured)

─── Error Information ───
Error: <message>

═══════════════════════════════════════════════════════════
End of Diagnostic Report
═══════════════════════════════════════════════════════════
```

## Troubleshooting Failed Tests

### Common failure modes

**1. Generator failed with non-zero exit status**

- Check the artifact's generator script
- Verify prompt values are passed correctly (look at the prompt file names in
  the diagnostic report)
- Check the generator's stderr in the diagnostic, if captured

**2. Backend configuration not found**

- Verify `backend.toml` exists in the scenario directory
- Check the TOML for syntax errors and that `check` / `serialize` are paired

**3. Serialization failed**

- Ensure the serialize script exists and is executable
- Verify backend storage paths under `ARTIFACTS_TEST_OUTPUT_DIR`

### Verbose logging

The CLI uses its own logger, not `RUST_LOG`. To get debug output from a
production run:

```bash
artifacts --log-file /tmp/artifacts.log --log-level debug
```

Test code constructs a `LogLevel` directly (see
`TestHarness::generate_artifact`, which currently uses `LogLevel::Info`).

## Snapshot Workflow

TUI view tests, CLI surface tests, and config parsing tests use `insta`. After a
code change that alters a snapshot:

```bash
cargo test --test tests …
cargo insta review
```

Do **not** run `cargo insta accept` or `cargo insta test --accept` unattended —
the project policy is to review snapshots manually before accepting.

## Test Requirements

E2E tests require:

- A working Nix installation with flake support (the harness calls `nix build`
  to materialise `make.json`)
- Scenarios under `pkgs/artifacts/examples/scenarios/`
- `serial_test`'s `#[serial]` attribute on tests that touch
  `ARTIFACTS_TEST_OUTPUT_DIR`
- `--test-threads=1` at the cargo command line

## CI Testing

CI runs the test suite with:

- Single-threaded execution (`--test-threads=1`)
- Serial test isolation (`#[serial]`)
- A Nix environment with flake support
- Diagnostic dumps on failure under `/tmp/artifacts_test_failures/`

## Adding New Tests

### E2E test template

```rust
use anyhow::Result;
use artifacts::app::model::TargetType;
use std::collections::BTreeMap;

use crate::common::{TestHarness, dump_test_diagnostics};

#[test]
#[serial_test::serial]
fn my_new_e2e_test() -> Result<()> {
    let harness = TestHarness::load_example("scenarios/my-scenario")?;

    let (_name, artifact_def) = harness
        .find_artifact("machine-name", None)
        .ok_or_else(|| anyhow::anyhow!("no artifacts for machine-name"))?;

    let target_type = TargetType::NixOS {
        machine: "machine-name".to_string(),
    };

    let prompts: BTreeMap<String, String> =
        BTreeMap::from([("prompt1".to_string(), "value1".to_string())]);

    let (result, diagnostics) = harness.generate_artifact_with_diagnostics(
        "machine-name",
        &artifact_def,
        target_type,
        &prompts,
    )?;

    if !result.success {
        let diag_dir = std::path::PathBuf::from("/tmp/artifacts_test_failures");
        let _ = std::fs::create_dir_all(&diag_dir);
        let _ = dump_test_diagnostics(
            &diagnostics,
            &diag_dir.join("my_new_e2e_test.txt"),
        );
        anyhow::bail!(
            "generation failed: {}",
            result.error.unwrap_or_default()
        );
    }

    Ok(())
}
```

## Related Files

- `tests/common/mod.rs` — `TestHarness`, `DiagnosticInfo`,
  `dump_test_diagnostics`
- `tests/e2e/diagnostics.rs` — Tests for the diagnostic system itself
- `tests/tests.rs` — Test entry point (declares all modules)
