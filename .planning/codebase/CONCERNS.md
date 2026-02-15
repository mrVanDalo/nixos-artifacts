# Codebase Concerns

**Analysis Date:** 2025-02-13

## Tech Debt

**Dual Data Structure Pattern (Legacy Migration):**

- Issue: Parallel data structures exist in TUI Model:
  `artifacts: Vec<ArtifactEntry>` (legacy) and `entries: Vec<ListEntry>` (new
  unified)
- Files: `pkgs/artifacts/src/app/model.rs`,
  `pkgs/artifacts/src/tui/model_builder.rs`
- Impact: Code complexity, potential for state inconsistency, maintenance burden
- Fix approach: Complete migration to unified `entries` field, remove legacy
  `artifacts` field (tracked in vibe-kanban task fca7bb48)

**unwrap()/expect() Usage:**

- Issue: Several `.unwrap()` and `.expect()` calls in non-test code
- Files:
  - `pkgs/artifacts/src/backend/output_capture.rs:31,32` -
    `expect("stdout not piped")` and `expect("stderr not piped")`
  - `pkgs/artifacts/src/tui/effect_handler.rs:127` -
    `expect("Serialize called without prior RunGenerator")`
  - `pkgs/artifacts/src/tui/effect_handler.rs:503` -
    `expect("SharedSerialize called without prior RunSharedGenerator")`
- Impact: Potential panics in production code instead of graceful error handling
- Fix approach: Replace with proper `Result` propagation using `?` operator and
  descriptive error contexts

**Test unwrap() Usage:**

- Issue: Extensive use of `.unwrap()` in test code (acceptable but could use
  `?`)
- Files: `pkgs/artifacts/src/config/backend.rs` (test functions),
  `pkgs/artifacts/src/backend/tempfile.rs` (test functions)
- Impact: Test failures show as panics rather than clean test failures
- Fix approach: Use `anyhow::Result` in tests and `?` operator for cleaner error
  propagation

**Large Module Files:**

- Issue: Several source files exceed 500 lines, potentially indicating too many
  responsibilities
- Files:
  - `pkgs/artifacts/src/app/update.rs` (968 lines) - Complex state machine
  - `pkgs/artifacts/src/config/backend.rs` (594 lines) - Backend configuration
    parsing
  - `pkgs/artifacts/src/config/make.rs` (571 lines) - Make configuration
    extraction
  - `pkgs/artifacts/src/tui/effect_handler.rs` (525 lines) - Effect execution
- Impact: Reduced maintainability, harder to understand, test, and modify
- Fix approach: Extract sub-modules by responsibility (e.g., separate
  check_serialization logic from effect handler)

## Known Issues

**Experimental Project Status:**

- Issue: README states "This project is currently in the design phase"
- Files: `README.md:14`
- Impact: APIs may change, documentation may be incomplete, features missing
- Status: Expected for early-stage project

**Clippy Warning:**

- Issue: Explicit auto-deref warning in tempfile.rs
- Files: `pkgs/artifacts/src/backend/tempfile.rs:312`
- Impact: Minor code style issue
- Fix approach: Replace `&*temp_file` with `&temp_file`

## Security Considerations

**Process Isolation:**

- Observation: Generator and serialize scripts run in bubblewrap container
  isolation
- Files: `pkgs/artifacts/src/backend/generator.rs`
- Current mitigation: Bubblewrap provides filesystem sandboxing
- Risk: Scripts still have access to `$out`, `$prompts`, and other temp
  directories
- Recommendations: Review bubblewrap arguments to ensure minimal privilege

**Temporary File Handling:**

- Observation: Custom `TempFile` implementation in `tempfile.rs` with manual
  cleanup in `Drop`
- Files: `pkgs/artifacts/src/backend/tempfile.rs`
- Current mitigation: Proper cleanup on drop, error handling for cleanup
  failures
- Risk: Concurrent runs may have PID conflicts in temp file naming (lines 27-28,
  54-55)
- Recommendations: Consider using UUID or random suffix instead of PID for
  uniqueness

**Script Execution:**

- Observation: Backend scripts (check_serialization, serialize, deserialize) are
  executed with user permissions
- Files: `pkgs/artifacts/src/backend/serialization.rs`
- Current mitigation: No elevated privileges, bubblewrap isolation
- Risk: Malicious scripts could read sensitive files if not properly sandboxed
- Recommendations: Audit all backend script invocations for proper argument
  escaping

## Performance Bottlenecks

**Nix Evaluation:**

- Observation: CLI evaluates flake.nix to extract configuration on every run
- Files: `pkgs/artifacts/src/config/nix.rs`
- Problem: Nix evaluation can be slow, especially for large flakes
- Improvement path: Cache evaluation results, use incremental evaluation

**Output Capture Threading:**

- Observation: `output_capture.rs` spawns threads for stdout/stderr reading
- Files: `pkgs/artifacts/src/backend/output_capture.rs`
- Problem: Thread spawning for every script execution adds overhead
- Improvement path: Consider using async I/O or a thread pool for frequently-run
  scripts

## Fragile Areas

**TUI State Machine:**

- Observation: Complex match-based state machine in `update.rs` with many
  screen/message combinations
- Files: `pkgs/artifacts/src/app/update.rs`
- Why fragile: Adding new screens requires updating multiple match arms, risk of
  unhandled combinations
- Safe modification: Add new variants to Screen and Msg enums first, then add
  handlers in update function
- Test coverage: State transition tests exist but snapshot tests need updating
  for UI changes

**Backend Configuration Parsing:**

- Observation: TOML parsing with custom include directive support
- Files: `pkgs/artifacts/src/config/backend.rs`
- Why fragile: Circular include detection, relative path resolution, duplicate
  backend name detection
- Safe modification: Add test cases for new configuration scenarios
- Test coverage: Unit tests exist for include parsing, circular detection

**Effect Handler State:**

- Observation: `current_out_dir` in `BackendEffectHandler` is Option<PathBuf>
  with temporal coupling
- Files: `pkgs/artifacts/src/tui/effect_handler.rs`
- Why fragile: `Serialize` effect assumes `RunGenerator` was called first
  (enforced by `.expect()`)
- Safe modification: Pass output directory explicitly in effect data rather than
  storing in handler
- Test coverage: Integration tests cover the happy path

## Scaling Limits

**Shared Artifact Generation:**

- Observation: Shared artifacts serialize to multiple targets sequentially
- Files: `pkgs/artifacts/src/tui/effect_handler.rs`
- Current capacity: Each target processed one at a time
- Limit: Large numbers of targets will take linear time
- Scaling path: Parallelize serialization across targets (requires careful error
  handling)

**Artifact List Rendering:**

- Observation: TUI list view renders all entries unconditionally
- Files: `pkgs/artifacts/src/tui/views/list.rs`
- Current capacity: All artifacts loaded into memory
- Limit: Very large numbers of artifacts could impact TUI responsiveness
- Scaling path: Implement virtualized/scrolled list view

## Dependencies at Risk

**ratatui 0.29:**

- Observation: TUI framework dependency
- Impact: Breaking changes between versions could require significant
  refactoring
- Migration plan: Pin version, review changelog before upgrading

**crossterm 0.28:**

- Observation: Terminal manipulation dependency
- Impact: Platform-specific bugs in terminal handling
- Migration plan: Test on multiple terminal emulators, consider crossterm
  upgrade path

## Missing Critical Features

**Backend Implementations:**

- Feature gap: Only test backend exists; production backends (agenix, sops-nix,
  colmena) not yet implemented
- Problem: Framework is not usable for real secrets management
- Blocks: Production deployment
- Status: Planned per README "(not yet)" markers

**Shared Artifact UI Display:**

- Feature gap: Shared artifacts not displayed in TUI list view
- Files: See `openspec/changes/missing-ui-elements/design.md`
- Problem: Users cannot see or interact with shared artifacts
- Blocks: Complete artifact management workflow
- Status: In progress (OpenSpec change created)

**Deserialization Workflow:**

- Feature gap: `deserialize` operation defined but not integrated into CLI
  workflow
- Files: `flake.nix:88-90` shows placeholder deserialize script
- Problem: Cannot restore artifacts from backend storage
- Blocks: Disaster recovery scenarios
- Status: Not yet implemented per README

## Test Coverage Gaps

**TUI Integration Tests:**

- What's not tested: Full end-to-end TUI workflows (key sequences, screen
  transitions)
- Files: `pkgs/artifacts/tests/tui/` has view snapshots but limited integration
  tests
- Risk: UI regressions not caught until manual testing
- Priority: Medium

**Error Path Testing:**

- What's not tested: Backend script failures, malformed configurations,
  permission errors
- Files: `pkgs/artifacts/tests/backend/`
- Risk: Error handling paths may have bugs
- Priority: High

**Shared Artifact Generation:**

- What's not tested: Complete shared artifact generation flow (currently not
  displayed in TUI)
- Files: `pkgs/artifacts/src/tui/effect_handler.rs` (shared methods)
- Risk: Shared artifact code paths may have bugs undetected
- Priority: High (blocks shared artifact feature)

---

_Concerns audit: 2025-02-13_
