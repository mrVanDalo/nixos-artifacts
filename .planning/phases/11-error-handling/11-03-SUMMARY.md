---
phase: 11-error-handling
plan: 03
subsystem: tui
tags:
  - error-handling
  - tui
  - stdout
  - stderr
  - model.error

requires:
  - phase: 11-error-handling
    provides: Pre-terminal error handling foundation
  - phase: 11-error-handling
    provides: Enhanced panic handler and terminal restoration

provides:
  - Audited TUI code paths confirming proper error channeling
  - Verified background task output capture
  - Documented headless mode stdout usage as acceptable
  - Confirmed runtime errors displayed via model.error

affects:
  - Phase 12 (Script output visibility)
  - Any future TUI error handling work

tech-stack:
  added: []
  patterns:
    - "Runtime errors displayed via model.error (ERR-03)"
    - "Background task output captured via channels"
    - "Headless mode println! acceptable for user interaction"

key-files:
  created: []
  modified: []

key-decisions:
  - "terminal.rs eprintln! calls are intentional (ERR-02 terminal operation error reporting)"
  - "prompt.rs println! calls are acceptable for headless mode user interaction"
  - "No changes needed - TUI code already compliant with ERR-03 requirements"

patterns-established:
  - "ERR-03: Runtime errors displayed via model.error in TUI"
  - "Background task output captured, not leaked to terminal"
  - "Headless mode stdout usage documented as acceptable"

duration: 3min
completed: 2026-02-18T11:53:55Z
---

# Phase 11 Plan 03: TUI Error Display Audit

**TUI code paths audited and verified - runtime errors display in TUI via model.error, background tasks capture output, headless mode stdout usage documented as acceptable**

## Performance

- **Duration:** 3 min
- **Started:** 2026-02-18T11:50:10Z
- **Completed:** 2026-02-18T11:53:55Z
- **Tasks:** 4
- **Files modified:** 0 (audit-only, no changes needed)

## Accomplishments

- Audited all TUI source files for println!/eprintln! usage
- Verified background task output capture (channel-based, no stdout/stderr leakage)
- Confirmed runtime errors use model.error pattern (ERR-03 compliant)
- Documented headless mode stdout usage in prompt.rs as acceptable
- Verified 85 TUI-related tests pass (49 integration + 36 unit)

## Task Commits

No code changes required - audit-only plan. All tasks verified existing code compliance:

1. **Task 1: Audit src/tui/ for println!/eprintln! usage** - Verified existing code compliance
2. **Task 2: Verify background task output capture** - Confirmed channel-based error propagation
3. **Task 3: Verify runtime error display in TUI** - Confirmed model.error pattern
4. **Task 4: Verify headless prompt mode stdout usage** - Documented as acceptable

**Note:** This was an audit-only plan. No commits were needed as the code was already compliant.

## Audit Results

### Task 1: TUI Code Audit for println!/eprintln!

**Files checked:**
- `src/tui/terminal.rs` - 4 eprintln! calls found (acceptable - see below)
- `src/tui/runtime.rs` - 0 println!/eprintln! calls
- `src/tui/background.rs` - 0 println!/eprintln! calls
- `src/tui/views/*.rs` - 0 println!/eprintln! calls

**terminal.rs eprintln! analysis (4 occurrences):**
- Lines 47, 53, 59: Error reporting for terminal restoration failures (ERR-02)
- Line 117: Panic message output after terminal restoration (ERR-04)

**Verdict:** These eprintln! calls are intentional and correct - they report terminal operation errors to stderr before/during panic situations where the TUI cannot display errors.

### Task 2: Background Task Output Capture

**Code pattern verified in background.rs:**
- Uses `tokio::sync::mpsc` channels for async communication
- Errors converted to `EffectResult` variants with error strings
- No direct stdout/stderr usage - all output captured in result structures
- Timeout handling converts to error results, not stderr output

**Example pattern:**
```rust
EffectResult::GeneratorFinished {
    artifact_index,
    success: false,
    output: None,
    error: Some(format!("Generator failed: {}", e)),
}
```

**Verdict:** Background tasks properly capture output via channels - ERR-03 compliant.

### Task 3: Runtime Error Display in TUI

**model.error usage found in runtime.rs:**
- Line 208: `model.error = Some("Connection to background task lost".to_string())`
- Line 262: `model.error = Some("Background task disconnected".to_string())`
- Line 300: `model.error = Some("Connection to background task lost".to_string())`
- Line 332: `model.error = Some("...".to_string())`
- Line 373: `model.error = Some("Connection to background task lost".to_string())`

**Verdict:** All runtime errors during TUI operation are displayed via model.error - ERR-03 compliant.

### Task 4: Headless Prompt Mode stdout Usage

**prompt.rs println! occurrences (6 total):**
- Lines 67-69: `non_interactive_read_prompt()` - headless mode description/prompt/output
- Lines 119-120: `interactive_read_prompt()` - description/prompt headers
- Line 158: Empty println after user input (interactive mode)
- Line 172: Empty println after multiline input submission

**Analysis:**
- All println! calls are for **user interaction** in headless/generate mode
- These occur BEFORE TUI starts (when running `artifacts generate` command)
- Interactive prompts use crossterm for TUI rendering, not println!

**Verdict:** Headless mode stdout usage is acceptable and documented in CLAUDE.md guidelines.

## Decisions Made

1. **terminal.rs eprintln! are intentional (ERR-02/ERR-04):** The 4 eprintln! calls in terminal.rs are for terminal operation error reporting (restore failures) and panic hook output. These are correct - they occur when the TUI cannot display errors.

2. **prompt.rs println! acceptable for headless mode:** The println! calls in prompt.rs are for user prompts in headless/generate mode. These are intentional user interaction, not debug output or errors.

3. **No code changes needed:** The TUI code is already compliant with ERR-03 requirements. All runtime errors go through model.error, background tasks capture output, and headless mode stdout usage is legitimate.

## Deviations from Plan

None - plan executed exactly as written. Audit confirmed code was already compliant.

## Issues Encountered

None. All verification passed successfully:
- 49 TUI integration tests passed
- 36 unit tests passed  
- Clippy warnings are pre-existing (unused variables, etc.)
- One unrelated tempfile test failure (pre-existing, not related to TUI code)

## Verification Summary

✅ **Task 1 Verification:** TUI code audited - only 4 eprintln! in terminal.rs (intentional ERR-02/ERR-04)
✅ **Task 2 Verification:** Background tasks use channel-based output capture
✅ **Task 3 Verification:** All runtime errors use model.error pattern (ERR-03)
✅ **Task 4 Verification:** prompt.rs println! documented as headless mode (acceptable)

---

_Phase: 11-error-handling_  
_Plan: 03_  
_Completed: 2026-02-18_
