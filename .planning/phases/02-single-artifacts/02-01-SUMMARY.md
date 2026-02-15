---
phase: 02-single-artifacts
plan: 01
subsystem: backend
tags: [tokio, spawn_blocking, backend, serialization, generator]

# Dependency graph
requires:
  - phase: 01-foundation
    provides: [BackgroundEffectHandler, EffectCommand, EffectResult, channel architecture]
provides:
  - Real CheckSerialization execution via spawn_blocking
  - Real RunGenerator execution with temp directories
  - Real Serialize execution consuming generator output
  - Temp directory lifecycle management across effects
affects:
  - 02-02: Shared Artifacts
  - 02-03: Error Handling

# Tech tracking
tech-stack:
  added: [tempfile crate as regular dependency]
  patterns:
    - "spawn_blocking for blocking I/O in async context"
    - "TempDir ownership preserved across effect boundaries"
    - "Fail-open error handling for check_serialization"

key-files:
  created: []
  modified:
    - pkgs/artifacts/src/tui/background.rs - Real backend integration for all three effects
    - pkgs/artifacts/src/backend/output_capture.rs - Added to_string() method to CapturedOutput
    - pkgs/artifacts/src/config/make.rs - Added Clone derive to MakeConfiguration
    - pkgs/artifacts/Cargo.toml - Moved tempfile from dev-dependencies to dependencies

key-decisions:
  - "Use spawn_blocking for all blocking operations to maintain TUI responsiveness"
  - "Store TempDir in handler struct to preserve it across RunGenerator -> Serialize"
  - "Fail-open on check_serialization errors - assume generation needed"
  - "Clone configuration data before moving into spawn_blocking closure"

patterns-established:
  - "Real backend integration: Look up artifact from config, call backend functions"
  - "Temp directory management: Create in RunGenerator, consume in Serialize, auto-cleanup"
  - "Error handling: Log errors at warn/error level, return appropriate EffectResult variants"

# Metrics
duration: 4m
completed: 2026-02-13
---

# Phase 02-01: Real Backend Integration Summary

**Replaced stub implementations with actual backend operations wrapped in
spawn_blocking**

This plan implemented the real backend integration for the three core
single-artifact effects: CheckSerialization, RunGenerator, and Serialize. Each
effect now executes actual scripts via tokio::task::spawn_blocking, maintaining
TUI responsiveness while performing blocking I/O.

## Performance

- **Duration:** 4m
- **Started:** 2026-02-13T20:43:05Z
- **Completed:** 2026-02-13T20:47:53Z
- **Tasks:** 3
- **Files modified:** 4

## Accomplishments

- **CheckSerialization** - Looks up artifact definition from make config, calls
  `backend::serialization::run_check_serialization()` in spawn_blocking, returns
  needs_generation flag and captured output. Fails open (assumes generation
  needed on error).

- **RunGenerator** - Creates tempfile::TempDir for output, creates prompts
  directory, writes prompts to files, looks up artifact, spawns blocking task to
  run `backend::generator::run_generator_script()` in bwrap container, verifies
  generated files, stores temp_dir and prompts in handler for Serialize.

- **Serialize** - Takes output directory from handler (set by RunGenerator),
  looks up artifact, spawns blocking task to run
  `backend::serialization::run_serialize()`, temp directory auto-cleaned on
  drop.

## Task Commits

Each task was committed atomically:

1. **Task 1: Implement CheckSerialization** - `34dd0bd` (feat)
2. **Task 2: Implement RunGenerator** - `89133b8` (feat)
3. **Task 3: Implement Serialize** - `af81240` (feat)

## Files Created/Modified

- `pkgs/artifacts/src/tui/background.rs` - Core implementation of all three
  effects
  - Added imports for HashMap, backend module
  - Added current_output_dir and current_prompts fields to
    BackgroundEffectHandler
  - Implemented CheckSerialization with spawn_blocking
  - Implemented RunGenerator with temp directory creation
  - Implemented Serialize consuming generator output

- `pkgs/artifacts/src/backend/output_capture.rs` - Added `to_string()` method to
  CapturedOutput for serializing output to EffectResult

- `pkgs/artifacts/src/config/make.rs` - Added `#[derive(Clone)]` to
  MakeConfiguration to enable cloning into spawn_blocking closures

- `pkgs/artifacts/Cargo.toml` - Moved tempfile from dev-dependencies to regular
  dependencies (needed for runtime temp directory creation)

## Decisions Made

- **spawn_blocking for all blocking I/O** - Required because
  run_check_serialization, run_generator_script, and run_serialize all execute
  subprocesses (scripts) which are blocking operations. Never call blocking
  operations directly in async context.

- **Temp directory ownership transfer** - The tempfile::TempDir must survive
  from RunGenerator completion until Serialize starts. Solution: Store it in the
  handler struct, then take() it in Serialize.

- **Fail-open for check_serialization** - If the check script fails or returns
  an error, we assume generation is needed. This is safer than skipping
  generation when we're not sure.

- **Clone before spawn_blocking** - Configuration data (backend, make,
  artifact_name, etc.) must be cloned before moving into the spawn_blocking
  closure to avoid borrow checker issues with async/await.

## Deviations from Plan

None - plan executed exactly as written. All three effects implemented as
specified.

## Issues Encountered

- **Cargo.toml tempfile dependency** - tempfile was only in dev-dependencies,
  causing compilation error. Fixed by moving to regular dependencies.

- **Clone derive missing** - MakeConfiguration didn't implement Clone, needed
  for use in spawn_blocking. Added #[derive(Clone)].

- **CapturedOutput Display trait** - EffectResult expects output as
  Option<String>, but CapturedOutput didn't have to_string(). Added method to
  convert lines to string.

- **Move semantics in closures** - Multiple iterations to correctly handle
  clone() vs move semantics when passing data into spawn_blocking closures.

## Next Phase Readiness

- All single-artifact effects now execute real backend operations
- Background task properly manages temp directory lifecycle
- Foundation complete for shared artifacts (Phase 02-02)
- No blockers for continuing to Phase 02-02

---

_Phase: 02-single-artifacts_ _Completed: 2026-02-13_
