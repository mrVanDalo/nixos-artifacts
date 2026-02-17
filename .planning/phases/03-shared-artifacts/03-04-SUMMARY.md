---
phase: 03-shared-artifacts
plan: 04
subsystem: TUI
completed: 2026-02-14
commits:
  - d5d8034
  - 31b5fd6
  - 484bccb
---

# Phase 03 Plan 04: Timeout Handling for Background Tasks - Summary

## What Was Accomplished

Fixed TUI freeze during artifact generation by adding comprehensive timeout
handling to prevent hanging scripts from blocking the UI. The implementation
spans three layers:

### 1. output_capture.rs - Low-level Timeout (Task 1)

- Added `ScriptError` enum with `Timeout`, `Failed`, and `Io` variants
- Created `run_with_captured_output_and_timeout` function
- Implemented timeout-based process termination with SIGKILL
- Added proper process cleanup to prevent zombie processes
- Stream output collection with `recv_timeout` for responsive termination

### 2. serialization.rs - Script-level Timeout (Task 2)

- Added `SERIALIZATION_TIMEOUT` constant (30 seconds)
- Updated `run_check_serialization` and `run_shared_check_serialization`
  - Timeout errors fail open (assume generation needed)
  - Other errors fail open with appropriate messages
- Updated `run_serialize` and `run_shared_serialize`
  - Timeout errors propagated as anyhow errors
  - Clear error messages distinguish timeout from other failures

### 3. background.rs - Task-level Timeout (Task 3)

- Added `BACKGROUND_TASK_TIMEOUT` constant (35 seconds: 30s script + 5s buffer)
- Wrapped all 6 spawn_blocking operations with tokio::time::timeout:
  - CheckSerialization
  - RunGenerator
  - Serialize
  - SharedCheckSerialization
  - RunSharedGenerator
  - SharedSerialize
- Updated all error handling match expressions to include timeout branch
- Timeout errors logged with artifact name for debugging
- UI receives "Timed out after 35 seconds" message

## Key Design Decisions

1. **Two-level timeout architecture**: Script-level (30s) kills hung scripts,
   task-level (35s) catches edge cases
2. **Fail-open for check operations**: Timeout during check assumes generation
   needed
3. **Fail-closed for serialize**: Timeout during serialize reports failure to
   user
4. **Process cleanup**: SIGKILL followed by wait() ensures no zombies
5. **Error propagation**: ScriptError::Timeout mapped to user-friendly messages

## Files Modified

1. `pkgs/artifacts/src/backend/output_capture.rs` - Timeout-aware process
   execution
2. `pkgs/artifacts/src/backend/serialization.rs` - Timeout-wrapped serialization
3. `pkgs/artifacts/src/tui/background.rs` - Timeout handling in background tasks

## Verification Results

- ✅ cargo check passes with no errors
- ✅ cargo clippy passes (only pre-existing warnings)
- ✅ cargo test --lib passes (94 tests)
- ✅ Compilation successful

## Deviation Documentation

**None** - Plan executed exactly as written.

## Integration Notes

The timeout implementation integrates seamlessly with existing error handling:

- TUI displays timeout errors as artifact failure
- Animation stops on timeout
- User can navigate and quit after timeout
- No breaking changes to public APIs

## Performance Impact

- **Positive**: Scripts that previously hung indefinitely now terminate after
  30-35 seconds
- **Negative**: None (timeout only affects hanging scripts)
- **Neutral**: Normal scripts complete unchanged within timeout window
