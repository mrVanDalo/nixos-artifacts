---
phase: 10-smart-generator-selection
plan: 01
subsystem: ui

tags:
  - shared-artifacts
  - generator-selection
  - smart-ux

requires:
  - phase: 09-shared-artifact-status
    provides: Shared artifact status tracking infrastructure

provides:
  - Smart generator selection for single-generator shared artifacts
  - Generator comparison by Nix store path
  - UX optimization skipping unnecessary dialogs

affects:
  - Phase 10 Plan 02 (enhanced dialog context)

tech-stack:
  added: []
  patterns:
    - "Smart selection: automatic choice when only one option exists"
    - "Generator path deduplication via make.rs"

key-files:
  created: []
  modified:
    - pkgs/artifacts/src/app/update.rs
    - pkgs/artifacts/src/app/update.rs (tests)

key-decisions:
  - "Single generator check: shared.info.generators.len() == 1 before showing dialog"
  - "Flow decision: if single generator, go to Prompt if prompts exist, else directly to Generating"
  - "Generator path storage: Set selected_generator in shared entry for consistency"

patterns-established:
  - "Smart selection: Check count before showing selection UI, skip when n=1"
  - "Early return pattern: Clone needed data before mutable borrow to avoid borrow checker issues"

duration: 25 min
completed: 2026-02-18
---

# Phase 10 Plan 01: Smart Generator Selection Summary

**Smart generator selection that skips the selection dialog when only one unique
generator exists for a shared artifact, streamlining the UX by 1-2 keypresses
per generation.**

## Performance

- **Duration:** 25 min
- **Started:** 2026-02-18T10:45:37Z
- **Completed:** 2026-02-18T10:47:00Z
- **Tasks:** 3
- **Files modified:** 1

## Accomplishments

- Modified `handle_shared_artifact_enter` in update.rs to check generator count
  before showing dialog
- Single generator flows directly to Prompt screen (or Generating if no prompts)
- Multiple generators still show SelectGenerator screen as before
- Generator path is stored in shared entry for consistency with multi-generator
  flow
- Added 4 comprehensive unit tests covering all code paths

## Task Commits

Each task was committed atomically:

1. **Task 1: Add generator count check before showing dialog** - `da0cb5e`
   (feat)
2. **Task 2: Add unit tests for generator selection logic** - `13a494d` (test)
3. **Task 3: Verify existing tests still pass** - Implicit (verification step,
   no code changes)

**Plan metadata:** (to be committed)

## Files Created/Modified

- `pkgs/artifacts/src/app/update.rs` - Added smart selection logic in
  `handle_shared_artifact_enter` (lines 168-246)
- `pkgs/artifacts/src/app/update.rs` - Added 4 unit tests:
  - `test_single_generator_skips_dialog`
  - `test_single_generator_no_prompts_goes_to_generating`
  - `test_multiple_generators_shows_dialog`
  - `test_single_generator_stores_selected_path`

## Decisions Made

- Used `shared.info.generators.len() == 1` as the check condition
- Clone needed data (files, targets, prompts) before mutable borrow to avoid
  Rust borrow checker issues
- Store the selected generator path in `shared.selected_generator` for
  consistency
- Preserve the same effect structure (`RunSharedGenerator`) for both
  smart-selected and user-selected flows

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- **Borrow checker issue:** Initial implementation tried to access
  `shared.info.files` after a mutable borrow of `model.entries`. Fixed by
  cloning all needed data before the mutable borrow.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Smart generator selection complete
- Tests verify both single and multiple generator paths
- Ready for Phase 10 Plan 02: Enhanced dialog context for when multiple
  generators exist

---

_Phase: 10-smart-generator-selection_ _Completed: 2026-02-18_
