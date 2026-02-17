---
phase: 03-shared-artifacts
plan: 06
type: fix
wave: 1

must_haves:
  truths:
    - "TUI remains responsive during all operations"
    - "Background task results are never starved"
    - "Unified event queue processes messages from all sources"
  artifacts:
    - path: "pkgs/artifacts/src/tui/runtime.rs"
      provides: "Unified event queue with Mutex-protected VecDeque"
      min_lines: 50
  key_links:
    - from: "event_queue Mutex"
      to: "background results channel"
      via: "drain phase"
      pattern: "timeout(Duration::from_secs(0), res_rx.recv())"
---

# Phase 03 Plan 06: Unified Event Queue Fix Summary

**Fixed TUI freeze with Mutex-protected unified event queue**

## Problem

The previous spawn_blocking fix had a subtle issue: `tokio::select!` with two
branches (event_rx and res_rx) can still cause starvation. When the event thread
is actively reading from crossterm (which blocks for up to 50ms), the `select!`
can't check the results channel.

## Solution

Implemented a **unified event queue** using `Arc<Mutex<VecDeque<Msg>>>`:

1. **Terminal Event Thread**: Reads crossterm events and pushes to the queue
2. **Drain Phase**: Non-blocking check of both:
   - Terminal events (via `queue.lock().unwrap().pop_front()`)
   - Background results (via `timeout(Duration::from_secs(0), res_rx.recv())`)
3. **Blocking Phase**: `select!` waits for either:
   - A background result (then push to queue and drain)
   - A 50ms timeout (continue draining)

This ensures:

- Background results are checked immediately (timeout=0)
- Events are never dropped
- No source can starve the other

## Key Changes

```rust
// Unified queue for all events
let event_queue: Arc<Mutex<VecDeque<Msg>>> = Arc::new(Mutex::new(VecDeque::new()));

// Event thread pushes to queue
while let Some(msg) = event_source.next_event() {
    queue.lock().unwrap().push_back(msg);
}

// Main loop drains before blocking
while had_events {
    // Check queue
    if let Some(msg) = queue.lock().unwrap().pop_front() { ... }
    
    // Check results (non-blocking)
    if let Ok(Some(result)) = timeout(Duration::from_secs(0), res_rx.recv()).await { ... }
}
```

## Performance

- **Duration**: 15 min
- **Files Modified**: 1
- **Tests**: 93/94 pass (1 pre-existing failure)

## Verification

- cargo check: Pass
- cargo clippy: Pass
- cargo test --lib: 93/94 pass
- Manual test: TUI remains responsive during generation
