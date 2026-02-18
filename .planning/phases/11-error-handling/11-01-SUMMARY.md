---
phase: 11-error-handling
plan: 01
subsystem: cli

tags:
  - error-handling
  - logging
  - stderr
  - anyhow

requires:
  - phase: 10-smart-generator-selection
    provides: Smart generator selection with conditional dialog display

provides:
  - Pre-terminal config loading with error context
  - Conditional output based on --log-file flag
  - Proper error propagation to stderr
  - Non-zero exit codes on config failures

affects:
  - Phase 12 (Script output visibility)
  - Phase 13 (Enhanced generator dialog)
  - Any future error handling work

tech-stack:
  added: []
  patterns:
    - "with_context() for anyhow error messages"
    - "Pre-terminal config loading for ERR-01"
    - "Conditional output based on CLI flags for UI-03"

key-files:
  created: []
  modified:
    - pkgs/artifacts/src/cli/mod.rs

key-decisions:
  - "Config loading happens before TerminalGuard::new() to ensure errors print to stderr"
  - "Empty artifacts check uses logging when --log-file provided, stdout otherwise"
  - "with_context() provides clear error messages for config loading failures"

patterns-established:
  - "ERR-01: Config loading before terminal setup ensures errors go to stderr"
  - "UI-03: Conditional output based on --log-file flag"
  - "ERR-04: Panic hook installed before terminal initialization"

duration: 6min
completed: 2026-02-18T11:48:00Z
---

# Phase 11 Error Handling: Plan 01 - Pre-terminal Error Handling

**Pre-terminal error handling with contextual messages and conditional output based on --log-file flag**

## Performance

- **Duration:** 6 min
- **Started:** 2026-02-18T11:42:16Z
- **Completed:** 2026-02-18T11:48:00Z
- **Tasks:** 3
- **Files modified:** 1

## Accomplishments

- Config loading now happens BEFORE terminal initialization (ERR-01)
- Config loading errors include clear context messages using `with_context()`
- Empty artifacts message uses logging when `--log-file` provided (UI-03)
- Error propagation maintains non-zero exit codes via main.rs

## Task Commits

Each task was committed atomically:

1. **Task 1: Move config loading before terminal setup with error context** - `30c0144` (feat)

**Plan metadata:** `30c0144` (docs: complete plan)

## Files Created/Modified

- `pkgs/artifacts/src/cli/mod.rs` - Restructured run_tui() function:
  - Added `cli: &args::Cli` parameter for accessing CLI flags
  - Moved `BackendConfiguration::read_backend_config()` before terminal setup with `with_context()` error messages
  - Moved `MakeConfiguration::read_make_config()` before terminal setup with `with_context()` error messages
  - Empty artifacts check now uses conditional output (logging vs stdout)
  - Panic hook installed before terminal initialization (ERR-04)
  - Moved logging startup message after terminal initialization

## Decisions Made

1. **Config loading before terminal setup:** Config loading happens before `TerminalGuard::new()` to ensure errors are printed to stderr in plain text before any TUI initialization. This satisfies ERR-01 requirement.

2. **Conditional output for UI-03:** When `--log-file` is provided, normal messages like "No artifacts found" go to the log file via `info!()` macro, not stdout. When no log file is provided, messages print to stdout.

3. **Error context with with_context():** Used `anyhow::Context::with_context()` to provide clear error messages indicating what failed (backend.toml vs nix evaluation).

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None. All verification passed successfully.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Phase 11, Plan 02 ready to implement
- Phase 12 (Script output visibility) can build on this error handling foundation
- Phase 13 (Enhanced generator dialog) can leverage the logging infrastructure

## Verification Summary

✅ **ERR-01 Verification:** Config loading happens before terminal setup with clear error context
✅ **UI-03 Verification:** Empty artifacts message conditionally outputs to stdout vs log file
✅ **ERR-04 Verification:** Panic hook installed before terminal initialization

---

_Phase: 11-error-handling_ _Completed: 2026-02-18_
