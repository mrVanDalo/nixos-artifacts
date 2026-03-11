---
phase: 11-error-handling
plan: 02
subsystem: error-handling
tags: [rust, tui, panic-hook, terminal, stderr, error-reporting]

requires:
  - phase: 11-error-handling
    provides: Pre-terminal error handling foundation

provides:
  - Enhanced panic hook that prints to stderr before calling original hook
  - TerminalGuard::restore() with per-step error reporting to stderr
  - Documented infallible restore_terminal() function
  - Terminal state restoration before any panic output

affects:
  - Phase 11-error-handling (remaining error handling tasks)
  - Any future terminal state management

tech-stack:
  added: []
  patterns:
    - "Panic hook restores terminal before output (ERR-04)"
    - "Explicit error reporting for terminal operations (ERR-02)"
    - "Infallible cleanup functions for panic safety"

key-files:
  created: []
  modified:
    - pkgs/artifacts/src/tui/terminal.rs - Enhanced panic hook and error-reporting restore

key-decisions:
  - "Panic hook restores terminal FIRST, then prints to stderr, then calls original hook"
  - "restore_terminal() is infallible - ignores all errors for panic safety"
  - "TerminalGuard::restore() tries all steps even if some fail, reporting each error"

patterns-established:
  - "Error reporting to stderr for terminal operations"
  - "Infallible cleanup functions for panic contexts"
  - "Terminal restoration before any output in panic handlers"

duration: 2min
completed: 2026-02-18
---

# Phase 11 Plan 02: Enhanced Panic Handler and Terminal Restoration

**Enhanced panic hook that restores terminal before printing errors to stderr,
with per-step error reporting in TerminalGuard::restore()**

## Performance

- **Duration:** 2 min
- **Started:** 2026-02-18T11:46:33Z
- **Completed:** 2026-02-18T11:48:32Z
- **Tasks:** 3
- **Files modified:** 1

## Accomplishments

- Panic hook now prints formatted error messages to stderr before calling
  original hook
- Panic hook restores terminal state FIRST (raw mode disabled, alternate screen
  exited) before any output
- TerminalGuard::restore() prints specific error messages for each failing
  restoration step
- TerminalGuard::restore() tries all three restoration steps even if some fail
  (ERR-02)
- Documented restore_terminal() as infallible and safe for panic hook use

## Task Commits

Each task was committed atomically:

1. **Task 1: Enhance panic hook to print to stderr** - `9741eb9` (feat)
2. **Task 2: Add error reporting to TerminalGuard::restore()** - `9d3f7fc`
   (feat)
3. **Task 3: Make restore_terminal() infallible** - `23fd436` (docs)

**Plan metadata:**
`docs(11-02): complete panic handler and terminal restoration plan` (pending)

## Files Created/Modified

- `pkgs/artifacts/src/tui/terminal.rs` - Enhanced panic hook, error-reporting
  restore, and documentation

## Decisions Made

- **Panic hook restoration order:** Terminal is restored FIRST (before any
  output), then error is printed to stderr, then original hook is called for
  backtrace. This ensures the terminal is usable for error output.
- **Payload handling:** Support both `&str` and `String` panic payloads, with
  fallback to "Unknown panic occurred" for other types.
- **Location reporting:** Include file and line number in panic message when
  available via `panic_info.location()`.
- **Error accumulation:** TerminalGuard::restore() tries all three steps (raw
  mode, alternate screen, cursor) even if earlier steps fail, reporting each
  error separately before returning.
- **Infallible cleanup:** restore_terminal() ignores all errors by design - it's
  meant for panic contexts where error handling isn't possible.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

One unrelated test failure detected during verification:

- `backend::tempfile::tests::test_temp_dir_creation` - File exists error (os
  error 17), appears to be a stale temp file collision not related to this
  plan's changes.

All TUI-specific tests (35 unit + 49 integration) pass successfully.

## Next Phase Readiness

- Terminal error handling infrastructure complete
- Ready for Phase 12: Script output visibility (UI-04, OUT-01-04)
- TUI foundation now has proper error reporting for:
  - Panic scenarios (terminal restoration + stderr output)
  - Terminal restoration failures (per-step error reporting)

---

_Phase: 11-error-handling_\
_Plan: 02_\
_Completed: 2026-02-18_
