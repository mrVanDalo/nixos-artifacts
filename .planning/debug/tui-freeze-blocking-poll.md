# Debug Session: TUI Freeze During Serialization

**Date:** 2026-02-14T13:50:00Z  
**Issue:** TUI becomes completely unresponsive during artifact serialization  
**Severity:** Blocker  
**Reporter:** User UAT retest

## Problem Description

When generating an artifact:
1. Generator step completes quickly (1-2 seconds)
2. Serialization step starts and shows ⟳ spinner
3. TUI **completely freezes** - cannot navigate with j/k
4. Ctrl-C and ESC don't work
5. Must close terminal window

## Initial Diagnosis (Wrong)

Original hypothesis: Serialization script hangs indefinitely, needs timeout handling.

**Plan 03-04 implemented:**
- Added ScriptError::Timeout variant
- Added run_with_captured_output_and_timeout() with 30s timeout
- Added BACKGROUND_TASK_TIMEOUT (35s) wrapping all spawn_blocking calls
- Result: Generator now times out correctly, but serialization still freezes TUI

## Root Cause Analysis

The real issue is in the **event polling loop**, not the serialization scripts.

### Code Path

File: `pkgs/artifacts/src/tui/runtime.rs` (lines 156-198)

```rust
// In run_async() main loop:
tokio::select! {
    // Branch 1: Handle terminal events
    msg = poll_next_event(events) => {  // <-- THIS BLOCKS
        // ...
    }
    
    // Branch 2: Handle background results  
    Some(result) = res_rx.recv() => {     // <-- Never reached while polling
        // ...
    }
}
```

File: `pkgs/artifacts/src/tui/runtime.rs` (lines 213-223)

```rust
async fn poll_next_event<E: EventSource>(events: &mut E) -> Option<Msg> {
    match tokio::time::timeout(tokio::time::Duration::from_millis(50), async {
        events.next_event()  // <-- BLOCKING CALL INSIDE async BLOCK
    })
    .await
    // ...
}
```

File: `pkgs/artifacts/src/tui/events.rs` (lines 36-47)

```rust
impl EventSource for TerminalEventSource {
    fn next_event(&mut self) -> Option<Msg> {
        if event::poll(self.tick_rate).ok()? {  // <-- BLOCKING SYNC CALL
            // ...
        }
    }
}
```

### The Bug

**The async block doesn't make the call non-blocking!**

When `poll_next_event()` is called:
1. It creates an async block containing `events.next_event()`
2. `events.next_event()` is a **synchronous** call to `crossterm::event::poll()`
3. `event::poll(Duration::from_millis(50))` **blocks the thread** for up to 50ms
4. During this time, the tokio thread cannot process other tasks (like channel results)
5. The `select!` can only check `res_rx.recv()` AFTER `poll_next_event()` completes
6. If the user isn't pressing keys, `poll()` waits the full 50ms before returning

**Result:** Even though serialization completes, the result sits in the channel until the event poll times out, creating the appearance of a freeze.

## Why Generator Works But Serialization Doesn't

**Generator:** Runs quickly (1-2s), returns before user notices  
**Serialization:** User expects immediate feedback, but blocked on event poll

The timeout in `poll_next_event()` (50ms) creates a **50ms latency** between serialization completing and the UI updating. This compounds with other delays.

## The Real Fix

The event polling needs to run on a **blocking thread pool** or use async-native input handling.

### Option 1: spawn_blocking (Recommended)

Wrap the blocking crossterm call in `tokio::task::spawn_blocking()`:

```rust
async fn poll_next_event<E: EventSource>(events: &mut E) -> Option<Msg> {
    // Run blocking crossterm call on blocking thread pool
    tokio::task::spawn_blocking(move || {
        events.next_event()
    })
    .await
    .ok()?
}
```

### Option 2: Async Event Source

Use tokio's async stdin handling or crossterm's async support (if available).

## Files to Modify

1. `pkgs/artifacts/src/tui/runtime.rs` - Change `poll_next_event()` to use spawn_blocking
2. `pkgs/artifacts/src/tui/events.rs` - May need to make EventSource methods async-compatible

## Verification Steps

1. Build with `cargo check`
2. Run TUI: `nix run .#artifacts`
3. Press Enter on artifact
4. **Verify:** Can navigate with j/k while generator runs
5. **Verify:** Can navigate while serialization runs
6. **Verify:** Can press ESC or Ctrl-C to exit during generation
7. **Verify:** No freeze at any point

## Related Issues

- Original gap: TUI freezes during generation
- Gap closure plan 03-04: Added timeouts to scripts
- **This issue:** Event polling blocks thread, preventing async operation

## References

- `pkgs/artifacts/src/tui/runtime.rs` - Main async runtime loop
- `pkgs/artifacts/src/tui/events.rs` - Terminal event source
- Crossterm docs: https://docs.rs/crossterm/latest/crossterm/event/
