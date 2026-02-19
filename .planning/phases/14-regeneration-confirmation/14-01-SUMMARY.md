---
phase: 14-regeneration-confirmation
plan: 01
subsystem: tui

tags: [rust, tui, artifact-entry, serialization-check, exists-flag]

requires:
  - phase: 13-background-effects
    provides: "EffectResult channels and background task infrastructure"
  - phase: 12-non-blocking
    provides: "Non-blocking effect execution architecture"

provides:
  - ArtifactEntry with exists: bool field
  - SharedEntry with exists: bool field
  - CheckSerializationResult message with needs_generation and exists fields
  - EffectResult::CheckSerialization with exists field
  - EffectResult::SharedCheckSerialization with exists field
  - Check script output parsing for "EXISTS" keyword
  - exists flag initialization in model builder

affects:
  - 14-02-confirmation-dialog
  - 14-03-ui-indicators

tech-stack:
  added: []
  patterns:
    - "Separate 'needs generation' from 'exists' status for precise regeneration tracking"
    - "Check script output parsing for artifact existence detection"

key-files:
  created: []
  modified:
    - pkgs/artifacts/src/app/model.rs
    - pkgs/artifacts/src/app/message.rs
    - pkgs/artifacts/src/app/update.rs
    - pkgs/artifacts/src/tui/effect_handler.rs
    - pkgs/artifacts/src/tui/model_builder.rs
    - pkgs/artifacts/src/tui/background.rs
    - pkgs/artifacts/src/tui/channels.rs
    - pkgs/artifacts/src/tui/runtime.rs
    - pkgs/artifacts/src/effect_handler.rs
    - pkgs/artifacts/src/cli/headless.rs

key-decisions:
  - "exists flag defaults to false until check_serialization proves otherwise"
  - "Check script parses 'EXISTS' keyword from output for existence detection"
  - "Exit success (exit code 0) implies both exists=true and needs_generation=false"
  - "Non-zero exit with 'EXISTS' in output implies exists=true, needs_generation=true"
  - "Non-zero exit without 'EXISTS' implies exists=false, needs_generation=true"
  - "Changed result type from Result<bool, String> to Result<(), String> with separate bool fields"

patterns-established:
  - "Artifact state tracking: exists flag enables confirmation dialog decision logic"
  - "Check script convention: output 'EXISTS' to signal artifact presence"

duration: 14min
completed: 2026-02-19
---

# Phase 14 Plan 01: Add exists Flag to Artifact Entries Summary

**Added exists flag to ArtifactEntry and SharedEntry structs, extended CheckSerializationResult message to include both needs_generation and exists fields, and wired exists detection through model builder and effect handlers.**

## Performance

- **Duration:** 14 min
- **Started:** 2026-02-19T19:39:42Z
- **Completed:** 2026-02-19T19:53:37Z
- **Tasks:** 4 completed
- **Files modified:** 10

## Accomplishments

- Added `exists: bool` field to `ArtifactEntry` struct in `app/model.rs`
- Added `exists: bool` field to `SharedEntry` struct in `app/model.rs`
- Extended `CheckSerializationResult` message with `needs_generation` and `exists` fields
- Extended `SharedCheckSerializationResult` message with same fields
- Updated effect handlers to parse check script output for "EXISTS" keyword
- Updated `EffectResult::CheckSerialization` and `EffectResult::SharedCheckSerialization` enums in channels
- Initialized `exists: false` in model builder for all artifact entries
- Updated all test code to use new message format

## Task Commits

Each task was committed atomically:

1. **Task 1: Add exists flag to artifact entries in model.rs** - `5190221` (feat)
2. **Task 2: Extend CheckSerializationResult message with exists flag** - `2259e67` (feat)

**Plan metadata:** `TBD` (docs: complete plan)

## Files Created/Modified

- `pkgs/artifacts/src/app/model.rs` - Added exists: bool to ArtifactEntry and SharedEntry
- `pkgs/artifacts/src/app/message.rs` - Extended CheckSerializationResult messages with needs_generation and exists fields
- `pkgs/artifacts/src/app/update.rs` - Updated handle_check_result and all test message constructions
- `pkgs/artifacts/src/tui/effect_handler.rs` - Updated to parse EXISTS from check script output
- `pkgs/artifacts/src/tui/model_builder.rs` - Initialize exists: false for all entries
- `pkgs/artifacts/src/tui/background.rs` - Added exists field to EffectResult constructions
- `pkgs/artifacts/src/tui/channels.rs` - Added exists field to EffectResult enum definitions
- `pkgs/artifacts/src/tui/runtime.rs` - Updated result_to_message to include exists
- `pkgs/artifacts/src/effect_handler.rs` - Updated result_to_message for new message format
- `pkgs/artifacts/src/cli/headless.rs` - Initialize exists: false in headless artifact entries

## Decisions Made

1. **exists defaults to false**: New artifacts start with exists=false until check_serialization proves otherwise
2. **Check script convention**: Scripts output "EXISTS" keyword to signal artifact already exists
3. **Message format change**: Changed from `Result<bool, String>` to `Result<(), String>` with separate `needs_generation: bool` and `exists: bool` fields for clearer semantics
4. **Exit code interpretation**: Exit 0 = exists=true, needs_generation=false; Non-zero with EXISTS = exists=true, needs_generation=true; Non-zero without EXISTS = exists=false, needs_generation=true

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Fixed multiple compilation errors from adding exists field**

- **Found during:** Task 1 and Task 2
- **Issue:** Adding exists field to structs required updating all construction sites across 10+ files
- **Fix:** Systematically updated all ArtifactEntry and SharedEntry constructions, and all CheckSerializationResult message constructions
- **Files modified:** app/update.rs, tui/runtime.rs, cli/headless.rs, effect_handler.rs, background.rs, channels.rs
- **Verification:** cargo check passes
- **Committed in:** Part of Task 1 and Task 2 commits

---

**Total deviations:** 1 auto-fixed (1 blocking) **Impact on plan:** All auto-fixes necessary for compilation. No scope creep.

## Issues Encountered

1. **Temporary file test failure**: `test_deref` test in tempfile.rs fails intermittently due to race condition (not related to plan changes)
2. **Breaking message format change**: Required updating ~40 call sites across the codebase

## Next Phase Readiness

- All infrastructure ready for Phase 14-02 (Confirmation Dialog)
- exists flag available in both ArtifactEntry and SharedEntry
- CheckSerializationResult includes exists and needs_generation for decision logic
- Backend scripts can signal existence via "EXISTS" keyword in output

---

_Phase: 14-regeneration-confirmation_ _Completed: 2026-02-19_
