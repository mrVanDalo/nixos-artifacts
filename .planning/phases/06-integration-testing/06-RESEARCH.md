# Phase 06: Integration Testing - Research

**Researched:** 2026-02-16
**Domain:** Rust integration testing with insta_cmd and headless CLI API
**Confidence:** HIGH

## Summary

Integration testing for the artifacts CLI involves verifying that artifacts are actually created and stored correctly through end-to-end tests. The project already has a foundation with a `tests/e2e/` module containing `mod.rs` with 6 integration tests covering basic scenarios. The headless API (`src/cli/headless.rs`) provides programmatic artifact generation without TUI interaction.

The test requirements focus on:
1. Programmatic invocation without TUI (TEST-01)
2. Single artifact creation with simple config (TEST-02)
3. Verification that artifacts exist at expected backend locations (TEST-03)
4. Content format verification (TEST-04)
5. Coverage for single-machine and shared artifacts (TEST-05)
6. CI integration with proper failure handling (TEST-06)

**Primary recommendation:** Extend the existing `tests/e2e/` module with additional test scenarios for shared artifacts, backend verification, and proper cleanup. Leverage the `serial_test` crate for test isolation and `tempfile` for test directories.

## User Constraints (from CONTEXT.md)

### Locked Decisions

- Create new dedicated test scenario: `examples/scenarios/test-e2e-simple/`
- Configuration: single machine, single artifact, one file
- Generator produces deterministic value (no prompts, no randomness)
- Expected value known upfront (e.g., `"test-secret-123"`)
- Keep it minimal — the simplest possible artifact configuration
- Full validation required:
  1. **Existence** — Artifact file exists at expected path
  2. **Exact content** — File content matches expected deterministic value
  3. **File permissions** — Owner and group match configuration
  4. **Metadata** — Artifact name, file name, and paths are correct
- Tests must catch serialization errors, not just generation failures
- **Detailed diagnostics mode** on test failure:
  - Dump full configuration (backend.toml, make config)
  - Capture environment variables
  - List temp directory contents
  - Show backend script output/logs
  - Include generator stdout/stderr
- Failures must be actionable — developer can see exactly what went wrong
- **Fresh isolation per test:**
  1. Create temp folder using `tempfile::TempDir`
  2. Initialize git repo in temp folder (required for flake evaluation)
  3. Copy `flake.nix` and supporting files to temp folder
  4. Use relative paths for artifact storage (relative to flake root)
- **Artifact storage paths:**
  - Machines: `./artifacts/machines/<machine>/artifact/<filename>`
  - Users: `./artifacts/users/<user>/artifact/<filename>`
  - Shared: `./artifacts/shared/<filename>`
- Each test gets clean state — no shared state between tests
- Use `#[serial]` attribute to prevent parallel execution conflicts

### Claude's Discretion

- Exact naming conventions for test scenario
- Specific helper function designs in `tests/e2e/mod.rs`
- Exact diagnostic output format on failure
- How to structure the new scenario's generator script

### Deferred Ideas (OUT OF SCOPE)

None — discussion stayed within phase scope

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `insta_cmd` | 0.6 | CLI snapshot testing | Standard for Rust CLI testing, captures command output |
| `insta` | 1.43.1 | Snapshot assertions | Ecosystem standard, used by insta_cmd |
| `serial_test` | 3 | Test isolation | Prevents test interference when tests modify shared state |
| `tempfile` | 3 | Temporary directories | Secure, auto-cleanup, cross-platform |
| `anyhow` | 1 | Error handling | Standard error handling in Rust CLI apps |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `assert_cmd` | - | Command assertions | Alternative to insta_cmd for simpler assertions |
| `predicates` | - | Assertion predicates | Used with assert_cmd for complex assertions |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `insta_cmd` | `assert_cmd` | insta_cmd provides better snapshot workflow for CLI output |
| `serial_test` | Manual locking | serial_test is cleaner and purpose-built |
| `tempfile` | `/tmp` paths | tempfile handles cleanup and collision avoidance automatically |

## Architecture Patterns

### Recommended Project Structure

```
tests/
├── e2e/                    # End-to-end integration tests
│   ├── mod.rs              # Test entry point
│   ├── single_artifact.rs  # Single machine artifact tests
│   ├── shared_artifact.rs  # Shared artifact tests
│   └── backend_verify.rs   # Backend storage verification tests
├── backend/                # Backend operation unit tests
├── tui/                    # TUI view snapshot tests
└── tests.rs                # Test main entry
```

### Pattern 1: Headless Artifact Generation Test

**What:** Tests that invoke `generate_single_artifact()` from the headless module directly, bypassing TUI.

**When to use:** When testing artifact generation logic without terminal interaction.

**Example:**
```rust
// From existing tests/e2e/mod.rs
#[test]
#[serial]
fn e2e_single_artifact_is_created() -> Result<()> {
    let (backend, make_config) = load_example("scenarios/single-artifact-with-prompts")?;
    let (artifact_name, artifact_def) = find_first_artifact(&make_config, "machine-name")
        .ok_or_else(|| anyhow::anyhow!("No artifacts found"))?;

    let prompt_values: PromptValues = BTreeMap::from([
        ("secret1".to_string(), "test-secret-one".to_string()),
        ("secret2".to_string(), "test-secret-two".to_string()),
    ]);

    let result = generate_single_artifact(
        "machine-name",
        &artifact_def,
        &prompt_values,
        &backend,
        &make_config,
    )?;

    assert!(result.success);
    assert!(!result.generated_files.is_empty());
    Ok(())
}
```

### Pattern 2: Backend Storage Verification

**What:** Tests that verify artifacts are actually stored in the backend by checking filesystem or mock backend output.

**When to use:** When implementing TEST-03 and TEST-04 requirements.

**Example:**
```rust
/// Verify artifact exists at expected backend location
fn verify_backend_storage(
    artifact: &ArtifactDef,
    backend_storage_dir: &Path,
) -> Result<()> {
    for file_name in artifact.files.keys() {
        let expected_path = backend_storage_dir.join(file_name);
        assert!(
            expected_path.exists(),
            "Artifact file {} should exist in backend storage",
            file_name
        );
    }
    Ok(())
}
```

### Pattern 3: Shared Artifact Multi-Target Test

**What:** Tests that shared artifacts are properly generated once and referenced by multiple machines.

**When to use:** When implementing TEST-05 for shared artifacts.

**Example:**
```rust
#[test]
#[serial]
fn e2e_shared_artifact_across_machines() -> Result<()> {
    let (backend, make_config) = load_example("scenarios/shared-artifacts")?;
    
    // Generate for machine-one
    let result_one = generate_single_artifact(
        "machine-one",
        &shared_artifact_def,
        &prompts,
        &backend,
        &make_config,
    )?;
    
    // Verify same artifact works for machine-two
    // (shared artifacts should be stored once, referenced by both)
    Ok(())
}
```

### Anti-Patterns to Avoid

- **Test interdependence:** Tests should not share state; use `#[serial]` when tests touch the filesystem or flake evaluation
- **Manual temp directory creation:** Never use `/tmp/test-xyz`; always use `tempfile::TempDir`
- **Hardcoded paths:** Tests should work regardless of where the repo is cloned
- **Ignoring cleanup:** Even with tempfile, ensure any external resources are cleaned up

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| CLI output assertions | Manual string comparison | `insta_cmd` | Handles normalization, provides review workflow |
| Test isolation | Manual mutex/semaphore | `serial_test` | Cleaner syntax, better error messages |
| Temp directories | Manual `/tmp` creation | `tempfile` | Secure permissions, auto-cleanup, collision avoidance |
| Process spawning | `std::process::Command` directly | `insta_cmd` | Captures output, handles exit codes, snapshot integration |
| Environment setup | Manual env vars | `std::env::set_var` with caution | Prefer passing config directly to functions |

**Key insight:** The existing codebase already uses `serial_test` and `tempfile` correctly. Custom test frameworks would be unnecessary and harder to maintain.

## Common Pitfalls

### Pitfall 1: Flake Evaluation Caching

**What goes wrong:** Tests fail intermittently because flake evaluation is cached or because Nix locks interfere between parallel tests.

**Why it happens:** Nix evaluation uses locks and can cache results. Multiple tests running in parallel may conflict.

**How to avoid:** Use `#[serial]` attribute from `serial_test` crate for any test that evaluates flakes. Tests in `e2e/mod.rs` already use this pattern.

**Warning signs:** Tests pass individually but fail when run together; "resource busy" errors from Nix.

### Pitfall 2: Backend Storage Verification

**What goes wrong:** Tests verify that files exist in temp directories but don't actually verify they're stored in the backend.

**Why it happens:** The test backend may just copy files rather than truly serialize them. Real backends (agenix, sops-nix) encrypt and store differently.

**How to avoid:** For TEST-03, check the actual backend storage location, not just the temp out directory. The test backend should write to `$ARTIFACTS_TEST_OUTPUT_DIR` if set (see `serialization.rs:94-97`).

**Warning signs:** Tests pass but actual artifact files don't appear in expected backend locations.

### Pitfall 3: Shared Artifact Complexity

**What goes wrong:** Tests for shared artifacts don't account for the different serialization path (`shared_serialize` vs `serialize`).

**Why it happens:** Shared artifacts require different backend scripts and environment variables (`$machines`, `$users` JSON files).

**How to avoid:** Verify shared artifacts use the `shared_serialize` function path (see `serialization.rs:136-261`). Tests should cover both the shared generation flow and the per-machine reference.

**Warning signs:** Shared artifacts generate but aren't accessible by all target machines.

### Pitfall 4: Missing Test Cleanup

**What goes wrong:** Tests leave artifacts in backend storage, causing subsequent test runs to fail or give false positives.

**Why it happens:** Tests verify artifacts exist but don't clean up after themselves.

**How to avoid:** Use `TempDir` which auto-cleans, or explicitly clean backend storage in test teardown. The current test backend stores in temp directories that are cleaned automatically.

**Warning signs:** "File already exists" errors; tests pass on first run but fail on second.

## Code Examples

### Loading Example Scenarios

```rust
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
```

### Verifying File Content

```rust
/// Check if a file exists and contains expected content.
fn verify_file_content(path: &Path, expected_content: &str) -> Result<()> {
    if !path.exists() {
        return Err(anyhow::anyhow!(
            "Expected file does not exist: {}",
            path.display()
        ));
    }

    let actual_content = fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", path.display()))?;

    if actual_content != expected_content {
        return Err(anyhow::anyhow!(
            "File content mismatch at {}\nExpected: {:?}\nActual: {:?}",
            path.display(),
            expected_content,
            actual_content
        ));
    }

    Ok(())
}
```

### Programmatic Invocation Without TUI

```rust
// From headless.rs - the API already exists
pub fn generate_single_artifact(
    target: &str,
    artifact: &ArtifactDef,
    prompt_values: &PromptValues,
    backend: &BackendConfiguration,
    make_config: &MakeConfiguration,
) -> Result<HeadlessArtifactResult> {
    // Full pipeline: check -> generate -> serialize
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Manual CLI testing | `insta_cmd` snapshot testing | Already adopted | Automated CLI output verification |
| Parallel test execution | `#[serial]` for flake tests | Already adopted | Prevents Nix lock conflicts |
| Custom temp handling | `tempfile` crate | Already adopted | Secure, auto-cleanup |
| Headless API | Direct function calls to `cli::headless` | Recently added (05-validation) | Enables programmatic testing |

**Deprecated/outdated:**

- None identified - current stack is appropriate.

## Open Questions

1. **Backend Storage Verification for TEST-03**
   - What we know: The test backend supports `ARTIFACTS_TEST_OUTPUT_DIR` environment variable
   - What's unclear: Should tests verify actual file system storage or mock backend state?
   - Recommendation: For integration tests, verify actual file system state using temp directories

2. **Shared Artifact Test Coverage**
   - What we know: Shared artifacts exist and have their own serialization path
   - What's unclear: What specific scenarios need testing for shared artifacts?
   - Recommendation: Test single shared artifact generation, multiple machines referencing it, and proper handling when machines have different generators

3. **CI Integration**
   - What we know: Tests use `#[serial]` and `cargo test` works
   - What's unclear: Are there specific CI requirements for artifact testing (timeouts, resource limits)?
   - Recommendation: Document that tests require Nix and may need extended timeouts due to flake evaluation

## Sources

### Primary (HIGH confidence)

- `tests/e2e/mod.rs` - Existing integration test implementation
- `src/cli/headless.rs` - Headless API for programmatic testing
- `src/backend/serialization.rs` - Serialization operations with `ARTIFACTS_TEST_OUTPUT_DIR` support
- `Cargo.toml` - Dependency versions for `insta_cmd`, `serial_test`, `tempfile`
- `examples/scenarios/` - Test scenarios available for use

### Secondary (MEDIUM confidence)

- Existing patterns from `tests/tui/view_tests.rs` - Snapshot testing patterns
- `examples/backends/test/` - Test backend implementation showing how artifacts are stored

### Tertiary (LOW confidence)

- None - all findings verified from codebase

## Metadata

**Confidence breakdown:**

- Standard stack: HIGH - Already in use in the project
- Architecture: HIGH - Clear patterns from existing code
- Pitfalls: MEDIUM-HIGH - Inferred from code structure and common Rust testing patterns

**Research date:** 2026-02-16
**Valid until:** 2026-03-16 (30 days for stable Rust ecosystem)
