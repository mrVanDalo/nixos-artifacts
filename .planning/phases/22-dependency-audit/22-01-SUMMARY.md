---
phase: 22-dependency-audit
plan: 01
subsystem: infra
tags: [cargo, dependencies, audit, cargo-machete]

requires:
  - phase: 21-rust-documentation
    provides: Zero warnings baseline for dependency audit

provides:
  - Verified dependency usage report
  - Confirmed feature flag usage
  - Duplicate dependency analysis

affects: []

tech-stack:
  added:
    - cargo-machete (0.9.1) - Unused dependency detection
  patterns:
    - Optional feature flags for logging
    - Dev dependency separation

key-files:
  created: []
  modified:
    - pkgs/artifacts/Cargo.toml - Verified all dependencies used

key-decisions:
  - "Confirmed all 11 dependencies are actively used"
  - "Verified logging feature is properly feature-gated (63 sites)"
  - "Documented unavoidable transitive duplicate dependencies"

patterns-established:
  - "Dependency verification: Always use cargo-machete + manual verification"
  - "Feature flag tracking: Search for cfg(feature) sites to confirm usage"
  - "Duplicate analysis: Check if transitive (unavoidable) vs direct (fixable)"

duration: 8min
completed: 2026-02-23
---

# Phase 22 Plan 01: Dependency Audit Summary

**All dependencies verified via cargo-machete, feature flags confirmed active,
and transitive duplicates documented**

## Performance

- **Duration:** 8 min
- **Started:** 2026-02-23T14:26:47Z
- **Completed:** 2026-02-23T14:34:00Z
- **Tasks:** 3
- **Files modified:** 0 (verification only, no changes needed)

## Accomplishments

- Ran cargo-machete and confirmed **no unused dependencies** detected
- Verified **11 dependencies** are all actively used in source code
- Confirmed **logging feature** has 63 conditional compilation sites
- Analyzed duplicate dependencies - all are unavoidable transitive deps
- Documented baseline for dependency hygiene

## Task Commits

Each task was committed atomically:

1. **Task 1: Analyze dependency usage with cargo-machete** - `b7878e5` (chore)
2. **Task 2: Verify feature flag usage** - `b7878e5` (chore - same commit as
   Task 1, empty commit)
3. **Task 3: Check for duplicate dependencies** - `cb5e9f1` (chore)

**Plan metadata:** See above commits

## Files Created/Modified

- `pkgs/artifacts/Cargo.toml` - Verified all dependencies are actively used
  - 11 direct dependencies confirmed
  - 4 dev-dependencies confirmed
  - 1 optional feature (logging) confirmed

## Dependency Analysis

### Direct Dependencies (11)

All dependencies verified as used:

| Dependency | Version | Usage                                    |
| ---------- | ------- | ---------------------------------------- |
| clap       | 4       | CLI argument parsing with derive feature |
| anyhow     | 1       | Error handling throughout codebase       |
| serde      | 1       | Serialization with derive feature        |
| serde_json | 1       | JSON handling in config and backend      |
| toml       | 0.8     | TOML parsing for backend configuration   |
| which      | 6       | Finding nix and nix-shell binaries       |
| log        | 0.4     | Optional logging infrastructure          |
| crossterm  | 0.28    | Terminal input handling                  |
| ratatui    | 0.29    | TUI framework                            |
| tokio      | 1       | Async runtime with selective features    |
| tokio-util | 0.7     | CancellationToken for TUI                |
| tempfile   | 3       | Temporary file/directory management      |

### Feature Flags

**default:** [] (empty - no features by default)

**logging:** ["dep:log"]

- 63 `#[cfg(feature = "logging")]` sites found
- Used in CLI, config, backend, and TUI modules
- Properly feature-gated throughout

### Duplicate Dependencies

5 sets found, all **unavoidable transitive dependencies**:

1. **hashbrown v0.15.5 vs v0.16.1**
   - v0.15.5: ratatui → lru
   - v0.16.1: toml → toml_edit → indexmap
   - Cannot unify - different dependency trees

2. **linux-raw-sys v0.4.15 vs v0.11.0**
   - v0.4.15: rustix 0.38 (crossterm, which)
   - v0.11.0: rustix 1.1 (tempfile)
   - Cannot unify - different rustix major versions

3. **rustix v0.38.44 vs v1.1.3**
   - v0.38: crossterm and which require this
   - v1.1: tempfile and insta require this
   - Cannot unify - major version incompatibility

4. **unicode-width v0.1.14 vs v0.2.0**
   - v0.1.14: ratatui → unicode-truncate
   - v0.2.0: ratatui direct dependency
   - ratatui intentionally uses both for compatibility

## Decisions Made

- No dependencies need to be removed (all are used)
- No feature flags need to be removed (logging is properly used)
- No duplicate dependencies can be resolved via Cargo.toml changes
- Dependency tree is clean and maintainable

## Deviations from Plan

None - plan executed exactly as written. All tasks completed:

- ✓ cargo-machete ran successfully - no unused deps
- ✓ All feature flags verified as used
- ✓ Duplicate analysis completed - all are unavoidable

## Issues Encountered

None. The codebase has excellent dependency hygiene:

- No unused dependencies detected
- All features properly implemented
- Duplicate dependencies are transitive (unavoidable)

## Next Phase Readiness

- Phase 22-01 complete
- Dependency audit establishes baseline for future dependency management
- Ready for next phase or additional dependency-related work

---

_Phase: 22-dependency-audit_ _Completed: 2026-02-23_
