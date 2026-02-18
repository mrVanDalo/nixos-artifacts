---
phase: 12-script-output-visibility
plan: 01
subsystem: tui

tags: [script-output, channels, runtime, ScriptOutput]

requires:
  - phase: 11-error-handling
    provides: [TUI error handling infrastructure]

provides:
  - ScriptOutput struct for structured output
  - EffectResult with ScriptOutput fields
  - Complete result_to_message conversion
  - Background task output conversion

affects:
  - 12-script-output-visibility/12-02-PLAN.md
  - 12-script-output-visibility/12-03-PLAN.md
  - 12-script-output-visibility/12-04-PLAN.md

tech-stack:
  added: [ScriptOutput struct]
  patterns: [Structured output preservation, stdout/stderr separation]

key-files:
  created: []
  modified:
    - pkgs/artifacts/src/tui/channels.rs
    - pkgs/artifacts/src/tui/runtime.rs
    - pkgs/artifacts/src/tui/background.rs
    - pkgs/artifacts/src/effect_handler.rs
    - pkgs/artifacts/src/app/message.rs

key-decisions:
  - "ScriptOutput struct with stdout_lines and stderr_lines fields"
  - "EffectResult variants use ScriptOutput instead of Option<String>"
  - "result_to_message properly converts ScriptOutput to message types"
  - "Background task converts CapturedOutput to ScriptOutput using from_captured()"

patterns-established:
  - "ScriptOutput::from_captured(): Converts CapturedOutput to ScriptOutput"
  - "ScriptOutput::from_message(): Creates ScriptOutput from error messages"
  - "ScriptOutput::default(): Creates empty output for error cases"

duration: 23min
completed: 2026-02-18
---

# Phase 12: Plan 01 - Script Output Visibility Data Flow Pipeline

**Enhanced EffectResult types with structured ScriptOutput, complete result_to_message conversion, and full stdout/stderr preservation through channels**

## Performance

- **Duration:** 23 min
- **Started:** 2026-02-18T13:29:31Z
- **Completed:** 2026-02-18T13:52:30Z
- **Tasks:** 4
- **Files modified:** 5

## Accomplishments

- Added `ScriptOutput` struct with `stdout_lines` and `stderr_lines` fields to preserve structured script output
- Updated all `EffectResult` variants to use `ScriptOutput` instead of `Option<String>` for output fields
- Implemented complete `result_to_message()` conversion in runtime.rs and effect_handler.rs
- Added helper methods to ScriptOutput: `from_captured()`, `from_message()`, and `default()`
- Updated background.rs to convert `CapturedOutput` from script execution to `ScriptOutput`
- All message types (`CheckOutput`, `GeneratorOutput`, `SerializeOutput`) now properly receive output data

## Task Commits

Each task was committed atomically:

1. **Task 1: Add ScriptOutput struct to channels.rs** - `7c5f353` (feat)
2. **Task 2: Update EffectResult variants with ScriptOutput** - `94f0100` (feat)
3. **Task 3: Update runtime.rs result_to_message conversion** - `5f491dd` (feat)
4. **Task 4: Verify message.rs types support full output** - `5f491dd` (feat) - verified as part of task 3

**Plan metadata:** `5f491dd` (docs: complete plan)

## Files Created/Modified

- `pkgs/artifacts/src/tui/channels.rs` - Added ScriptOutput struct, updated EffectResult variants
- `pkgs/artifacts/src/tui/runtime.rs` - Complete result_to_message conversion
- `pkgs/artifacts/src/tui/background.rs` - ScriptOutput creation from CapturedOutput
- `pkgs/artifacts/src/effect_handler.rs` - Updated result_to_message for ScriptOutput
- `pkgs/artifacts/src/app/message.rs` - Verified output types (no changes needed)

## Decisions Made

1. **ScriptOutput Structure**: Used simple Vec<String> fields for stdout/stderr to match the split_captured_output() return type from effect_handler.rs
2. **Conversion Pattern**: Added `from_captured()` method to convert from CapturedOutput (lines with stream markers) to ScriptOutput (separate vectors)
3. **Error Handling**: `from_message()` creates ScriptOutput with message in stdout_lines for error cases
4. **Default Values**: Used `ScriptOutput::default()` for error paths where no actual output exists

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

None - all changes compiled successfully.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Script output data flow pipeline complete
- Ready for Plan 12-02: Script output display in TUI views
- Ready for Plan 12-03: Artifact detail view with output
- Ready for Plan 12-04: Gap closure verification

---

_Phase: 12-script-output-visibility_ _Completed: 2026-02-18_
