# Phase 06: Integration Testing - Context

**Gathered:** 2026-02-16 **Status:** Ready for planning

<domain>
## Phase Boundary

Create end-to-end tests that verify artifacts are actually created and stored correctly in the backend. Tests run programmatically without TUI, using a headless API. Coverage includes single-machine and shared artifacts, with full validation of existence, content, and metadata.

</domain>

<decisions>
## Implementation Decisions

### Test scenario selection

- Create new dedicated test scenario: `examples/scenarios/test-e2e-simple/`
- Configuration: single machine, single artifact, one file
- Generator produces deterministic value (no prompts, no randomness)
- Expected value known upfront (e.g., `"test-secret-123"`)
- Keep it minimal — the simplest possible artifact configuration

### Verification depth

- Full validation required:
  1. **Existence** — Artifact file exists at expected path
  2. **Exact content** — File content matches expected deterministic value
  3. **File permissions** — Owner and group match configuration
  4. **Metadata** — Artifact name, file name, and paths are correct
- Tests must catch serialization errors, not just generation failures

### Failure behavior

- **Detailed diagnostics mode** on test failure:
  - Dump full configuration (backend.toml, make config)
  - Capture environment variables
  - List temp directory contents
  - Show backend script output/logs
  - Include generator stdout/stderr
- Failures must be actionable — developer can see exactly what went wrong

### Test isolation strategy

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

</decisions>

<specifics>
## Specific Ideas

- Artifact storage should use relative paths from flake root: `./artifacts/machines/<machine>/artifact/<filename>`
- Generator should write deterministic content without requiring prompts
- New scenario should be in `examples/scenarios/test-e2e-simple/` with minimal flake.nix
- Git repo initialization is required because flake evaluation requires a git repository
- Each test starts fresh: new temp dir, new git repo, copied flake files

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

_Phase: 06-integration-testing_ _Context gathered: 2026-02-16_
