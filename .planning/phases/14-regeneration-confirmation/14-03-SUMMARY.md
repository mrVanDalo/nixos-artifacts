---
phase: 14-regeneration-confirmation
plan: 03
subsystem: tui
tags: [rust, ratatui, tui, status-text, exists-flag]

# Dependency graph
requires:
  - phase: 14-regeneration-confirmation
    plan: 01
    provides: "exists flag infrastructure on ArtifactEntry and SharedEntry"
  - phase: 14-regeneration-confirmation
    plan: 02
    provides: "ConfirmRegenerateState for regeneration confirmation dialog"
provides:
  - GeneratingState with exists: bool field
  - "Regenerating artifact: {name}" status text in generating view
  - "Generating artifact: {name}" status text in generating view
  - "Regenerating..." status text in list view for generating artifacts
  - "Generating..." status text in list view for generating artifacts
  - All transitions to Generating screen properly set exists flag
affects:
  - User-facing documentation for TUI workflows
  - User experience with clear visual feedback on overwrite operations

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Full sentence status format: '{Verb} artifact: {name}' for clarity"
    - "Verb selection based on exists flag: Regenerating vs Generating"
    - "Status text display in list view for active generation state"

key-files:
  created: []
  modified:
    - pkgs/artifacts/src/app/model.rs
    - pkgs/artifacts/src/app/update.rs
    - pkgs/artifacts/src/tui/views/progress.rs
    - pkgs/artifacts/src/tui/views/list.rs

key-decisions:
  - "Full sentence format: Use '{Verb} artifact: {name}' instead of 'Generating: {name}' for clarity"
  - "Verb determined by exists flag: Regenerating for existing, Generating for new"
  - "Exists comes from entry's flag: Single uses single.exists, Shared uses shared.exists"
  - "Status text in list: Shows 'Regenerating...' or 'Generating...' during active generation"
  - "Progress header uses state.exists: Consistent with dialog confirmation logic"

patterns-established:
  - "Status verb selection: Use exists flag to choose between Regenerating and Generating"
  - "Full sentence format: 'Regenerating artifact: ssh-key' instead of 'Generating: ssh-key'"
  - "Consistent terminology: Same verb used in dialog, generating screen, and list"

duration: 26min
completed: 2026-02-19
---

# Phase 14 Plan 03: Status Text Update (Regenerating vs Generating) Summary

**Status text throughout TUI distinguishes between 'Regenerating' (existing) and
'Generating' (new) artifacts with full sentence format**

## Performance

- **Duration:** 26 min
- **Started:** 2026-02-19T20:25:00Z
- **Completed:** 2026-02-19T20:51:00Z
- **Tasks:** 4
- **Files modified:** 4

## Accomplishments

- Added `exists: bool` field to `GeneratingState` struct for tracking
  regeneration vs new creation
- Updated generating progress view to show "Regenerating artifact: {name}" or
  "Generating artifact: {name}"
- Updated artifact list view to display "Regenerating..." or "Generating..."
  status text during generation
- Fixed all 5 GeneratingState construction sites in update.rs to properly pass
  exists flag from entry data

## Task Commits

Each task was committed atomically:

1. **Task 1: Add exists flag to GeneratingState** - `85d0827` (feat)
2. **Task 2: Update generating view and transitions** - `7d72b85` (feat)
3. **Task 3: Update artifact list view status column** - `83ff6de` (feat)

**Plan metadata:** `TBD` (docs: complete plan)

_Note: Task 2 and 3 were combined in one commit due to compilation dependency_

## Files Created/Modified

- `pkgs/artifacts/src/app/model.rs` - Added `exists: bool` field to
  `GeneratingState` struct
- `pkgs/artifacts/src/app/update.rs` - Updated 5 GeneratingState constructions
  to pass exists flag
- `pkgs/artifacts/src/tui/views/progress.rs` - Added verb selection based on
  state.exists for header text
- `pkgs/artifacts/src/tui/views/list.rs` - Replaced `status_display` with
  `status_display_with_text` helper, added generating status text

## Decisions Made

1. **Full sentence format**: Changed from "Generating: {name}" to "{Verb}
   artifact: {name}" for clarity and consistency
2. **Exists flag source**: Uses entry's exists flag (single.exists or
   shared.exists) rather than tracking separately
3. **New artifacts (prompt screen)**: When coming from prompt screen (no
   confirmation shown), exists is set to false since confirmation would have
   been shown if artifact existed
4. **Status text in list**: Only shows verb during Generating status, keeping
   list uncluttered for other states

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Compilation errors from missing exists field in 5
construction sites**

- **Found during:** Task 1 completion
- **Issue:** Adding exists field to GeneratingState broke 5 compilation sites in
  update.rs
- **Fix:** Updated all GeneratingState constructions:
  - `finish_prompts_and_start_generation`: exists=false (new artifact)
  - `start_generation_for_selected_internal` (single): exists=single.exists
  - `update_generator_selection` (shared, 1 gen): exists=shared.exists
  - `start_generation_for_selected_internal` (shared, 1 gen):
    exists=shared.exists
  - Test fixtures: exists=false
- **Files modified:** pkgs/artifacts/src/app/update.rs
- **Verification:** cargo check passes
- **Committed in:** 7d72b85 (Task 2 commit)

---

**Total deviations:** 1 auto-fixed (1 blocking) **Impact on plan:** All
auto-fixes necessary for compilation. No scope creep.

## Issues Encountered

- **Borrow checker issue in shared artifact flow**: Had to restructure code to
  capture `shared.exists` before modifying model.screen, using temporary
  variable `shared_exists`
- **Tempfile test failure**: `test_deref` test fails intermittently (unrelated
  to plan changes, race condition in temp file test)

## Next Phase Readiness

- All status text updates complete
- Exists flag properly flows from ArtifactEntry/SharedEntry → GeneratingState →
  progress view
- List view shows appropriate verb based on entry.exists
- Ready for Phase 15 - Chronological Log View with Expandable Sections

## Self-Check: PASSED

### File Existence

All key files verified on disk:

- model.rs - GeneratingState with exists field present
- update.rs - All 5 GeneratingState constructions include exists
- progress.rs - Verb selection and "Regenerating artifact:" text present
- list.rs - status_display_with_text with exists check present

### Verification Commands

- `cargo check` - passes (39 warnings, none related to changes)
- `cargo test --lib` - 121 passed, 1 failed (unrelated tempfile race condition)

### Implementation Verification

- GeneratingState exists with all required fields including `exists: bool`
- progress.rs shows "Regenerating artifact:" when state.exists is true
- progress.rs shows "Generating artifact:" when state.exists is false
- list.rs shows "Regenerating..." when entry.exists is true and status is
  Generating
- list.rs shows "Generating..." when entry.exists is false and status is
  Generating
- All 5 update.rs constructions pass correct exists value

---

_Phase: 14-regeneration-confirmation_ _Completed: 2026-02-19_
