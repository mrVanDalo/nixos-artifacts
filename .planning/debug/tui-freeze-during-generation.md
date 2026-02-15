# Debug Session: TUI Freeze During Generation

## Problem

**Test:** 3. Animated Spinner During Generation  
**Reported:** "TUI shows 'running generator' dialog with check symbol, serialization shows wait symbol, but then TUI stops responding entirely. Cannot navigate with j/k, Ctrl-C doesn't work, must close terminal window"

**Severity:** Blocker

## Root Cause Analysis

### Symptoms
- TUI freezes completely during artifact generation
- Cannot navigate with j/k
- Ctrl-C doesn't work
- Must close terminal window to recover
- Status shows "running generator" with check symbol
- Serialization shows wait symbol

### Code Flow Analysis

Looking at the runtime loop in `src/tui/runtime.rs`:

1. **Event Polling** (`poll_next_event`):
   - Uses `tokio::time::timeout` with 50ms
   - Should allow concurrent channel checking

2. **Background Task** (`spawn_background_task`):
   - Runs effects sequentially in FIFO order
   - Uses `tokio::task::spawn_blocking` for blocking operations

3. **Serialization Execution** (`src/backend/serialization.rs`):
   - Line 93-98: Spawns script with `cmd.spawn()`
   - Waits with `run_with_captured_output`
   - Called from `background.rs` wrapped in `spawn_blocking`

### Potential Causes

**Cause 1: Serialize Script Hanging**
- The serialize script might be waiting for input or stuck
- `run_with_captured_output` waits for child process completion
- If script never exits, the spawn_blocking task never completes
- This would block the background task queue

**Cause 2: Blocking I/O in Async Context**
- Even with `spawn_blocking`, if the script waits indefinitely,
- the background task is occupied and can't process new commands
- But TUI foreground should still process events...

**Cause 3: Event Loop Blocked**
- Something in the foreground is blocking
- Perhaps waiting for a result that never comes?
- But `tokio::select!` should handle this...

### Most Likely Root Cause

The serialize script is hanging. Looking at `run_serialize`:

1. It creates temp files (config.json)
2. Spawns the script with `sh`
3. Waits for completion with `run_with_captured_output`

If the serialize script has an issue (infinite loop, waiting for input, etc.),
the spawn_blocking call never returns, blocking the background task.

### Evidence Needed

Check `/tmp/artifacts_debug.log` if it exists to see what commands were sent
and what results were received.

## Fix Required

Need to add timeout handling to serialize operations in the background task.
Scripts should not be allowed to run indefinitely - they should have a reasonable
timeout (e.g., 30 seconds) after which they're terminated and an error is reported.

## Artifacts

- `pkgs/artifacts/src/tui/background.rs` - Background task execution
- `pkgs/artifacts/src/backend/serialization.rs` - Serialization script execution
- `pkgs/artifacts/src/backend/output_capture.rs` - Output capture logic

## Missing

- Timeout handling for script execution in spawn_blocking context
- Better error reporting when scripts hang
- Debug visibility into what script is currently running

## Test Reproduction Steps

1. Run: nix run .#artifacts
2. Select an artifact with ◐ status
3. Press Enter to generate
4. TUI freezes during serialization phase
5. Must close terminal to recover
