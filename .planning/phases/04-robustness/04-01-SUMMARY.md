---
phase: 04-robustness
plan: 01
subsystem: tui

tags:
  - tokio
  - tokio-util
  - cancellation-token
  - shutdown
  - graceful-exit

requires:
  - phase: 03-shared-artifacts
    provides: background task infrastructure with channel-based communication

provides:
  - Clean shutdown mechanism for background task
  - Cooperative cancellation via tokio-util::CancellationToken
  - FIFO-preserving shutdown with queued command processing

affects:
  - runtime.rs (spawning)
  - Any code that spawns background tasks

tech-stack:
  added:
    - tokio-util = "0.7" (full feature)
  patterns:
    - tokio::select! for multiple event sources
    - CancellationToken for cooperative cancellation
    - Graceful shutdown with command queue draining

key-files:
  created: []
  modified:
    - pkgs/artifacts/Cargo.toml - Added tokio-util dependency
    - pkgs/artifacts/src/tui/background.rs - Implemented CancellationToken shutdown
    - pkgs/artifacts/src/tui/runtime.rs - Updated to create shutdown_token

duration: 12min
completed: 2026-02-14
---

# Phase 04: Plan 01 - CancellationToken Shutdown Summary

**Implemented graceful shutdown of background task using tokio-util::CancellationToken, converting the while loop to a tokio::select! that listens for shutdown signals while maintaining FIFO command ordering.**

## Performance

- **Duration:** 12 min
- **Started:** 2026-02-14T18:02:27Z
- **Completed:** 2026-02-14T18:14:00Z
- **Tasks:** 5
- **Files modified:** 3

## Accomplishments

1. Added tokio-util dependency with full feature set to Cargo.toml
2. Updated spawn_background_task signature to accept CancellationToken parameter
3. Converted background loop from while to tokio::select! with cancellation branch
4. Implemented graceful shutdown: process queued commands, then exit cleanly
5. Updated runtime.rs to create shutdown_token and pass to spawn function
6. Fixed all test cases to use new 3-parameter signature

## Task Commits

1. **Task 1: Add tokio-util dependency** - `805a38a` (chore)
2. **Task 2: Update spawn_background_task with CancellationToken** - `ab88240` (feat)

**Plan metadata:** (to be committed)

## Files Created/Modified

- `pkgs/artifacts/Cargo.toml` - Added tokio-util = { version = "0.7", features = ["full"] }
- `pkgs/artifacts/src/tui/background.rs` - Added CancellationToken import, updated function signature, implemented tokio::select! loop with shutdown/cancellation/channel branches
- `pkgs/artifacts/src/tui/runtime.rs` - Added CancellationToken import, create shutdown_token when spawning background task

## Decisions Made

None - followed plan as specified.

## Deviations from Plan

None - plan executed exactly as written.

Task 4 (mod.rs exports) was not needed since CancellationToken is used internally and doesn't need to be exported through the module interface.

Task 5 (child token helper) was skipped as unnecessary - the current design processes effects sequentially without spawning sub-tasks, so child tokens aren't required.

## Issues Encountered

None

## Verification Results

- `cargo check` passes with no new errors (3 pre-existing warnings)
- Background task tests pass (3/3):
  - test_spawn_background_task_creates_channels ✓
  - test_fifo_ordering ✓
  - test_graceful_exit_on_channel_close ✓
- Integration: runtime.rs compiles and passes all tests

## Implementation Details

The shutdown mechanism works as follows:

1. **Foreground creates token:** `let shutdown_token = CancellationToken::new();`
2. **Passes to background:** `spawn_background_task(backend, make, shutdown_token)`
3. **Background listens:** `tokio::select!` with three branches:
   - `shutdown_token.cancelled()` - triggers graceful shutdown
   - `Some(cmd) = rx_cmd.recv()` - process next command
   - `else` - channel closed, exit
4. **On shutdown:** Process remaining queue commands via `try_recv()`, then exit

This ensures:
- **Cooperative cancellation:** Background task only exits when it chooses
- **FIFO preservation:** Commands are processed in order
- **Clean shutdown:** Temp directories are dropped properly (handler dropped at end)
- **No blocking:** select! allows immediate response to cancellation

## Next Phase Readiness

- Background task now supports explicit shutdown signaling
- Ready for SHUT-01: TUI exit can now signal background to stop
- Can add `shutdown_token.cancel()` call on TUI quit to ensure clean exit

---

_Phase: 04-robustness_ _Completed: 2026-02-14_
