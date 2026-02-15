# Phase 5: Validation — Testing - Research

**Researched:** 2026-02-14  
**Domain:** Async tokio channel testing, Elm Architecture with background jobs, Rust test patterns  
**Confidence:** HIGH

## Summary

Phase 5 focuses on updating all tests to work with the new async channel-based architecture where the TUI runs as foreground, effects execute in a background task, and communication happens via `tokio::sync::mpsc` channels. The research identifies patterns for:

1. **Testing async tokio channel-based code** — Channel-level mocks, time control, and sequential execution
2. **Mocking tokio mpsc channels** — Direct channel mocking vs handler abstraction mocking
3. **Elm Architecture with async background jobs** — Pure state transitions with async effect testing
4. **Achieving 80% coverage** — Critical path coverage for channels, select! branches, and error handling
5. **Testing tokio::select! branches** — Branch coverage strategies and cancellation safety
6. **Graceful shutdown testing** — Cooperative cancellation with CancellationToken

**Primary recommendation:** Use channel-level mocks (mocking the sender/receiver directly) for unit tests, employ `tokio::time::pause()` for timing tests, use `#[serial]` for async tests to prevent shared state conflicts, and prioritize snapshot testing for integration tests.

---

## Standard Stack

### Core Testing Libraries

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| tokio | 1.x | Async runtime, test runtime | Already in use, standard for Rust async |
| tokio-test | 0.4.x | Testing utilities, mock I/O | Official tokio testing companion |
| serial_test | 2.x | Sequential test execution | Prevents shared state conflicts in async tests |
| insta | 1.x | Snapshot testing | Already in use for view tests |
| insta_cmd | 0.6.x | CLI snapshot testing | Already in use for integration tests |

### Tokio Test Utilities

| Feature | Purpose | When to Use |
|---------|---------|-------------|
| `#[tokio::test]` | Async test attribute | All async tests |
| `#[tokio::test(start_paused = true)]` | Start with paused time | Timing/timeout tests |
| `tokio::time::pause()` | Pause time in tests | Tests with delays/timeouts |
| `tokio::time::advance()` | Manually advance time | Testing timeout behavior |
| `tokio_test::io::Builder` | Mock AsyncRead/AsyncWrite | I/O mocking |

### Test Organization

| Pattern | Location | Purpose |
|---------|----------|---------|
| Unit tests | `src/` inline (`#[cfg(test)]`) | Test private functions, fast feedback |
| Async unit tests | `tests/async/` | Async channel testing, sequential execution |
| Integration tests | `tests/` | End-to-end CLI testing with snapshots |
| Snapshot tests | `tests/**/snapshots/` | View rendering, CLI output verification |

---

## Architecture Patterns

### Pattern 1: Channel-Level Mocking

**What:** Mock the tokio mpsc channel directly (sender/receiver) rather than mocking higher-level abstractions.

**When to use:** For unit tests that verify correct message sending/receiving between foreground and background tasks.

**Example:**
```rust
// Source: Tokio testing best practices
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender, UnboundedReceiver};

#[tokio::test(start_paused = true)]
async fn test_background_job_receives_commands() {
    // Create real channels - the "mock" is controlling the other end
    let (cmd_tx, mut cmd_rx) = unbounded_channel::<EffectCommand>();
    let (res_tx, res_rx) = unbounded_channel::<EffectResult>();
    
    // Spawn background task with test channels
    let handle = tokio::spawn(async move {
        // Simulate background task reading commands
        while let Some(cmd) = cmd_rx.recv().await {
            // Process command and send result
            let result = process_command(cmd);
            res_tx.send(result).unwrap();
        }
    });
    
    // Send command from "foreground"
    cmd_tx.send(EffectCommand::RunGenerator { artifact_index: 0 }).unwrap();
    
    // Verify response received
    let result = tokio::time::timeout(Duration::from_secs(1), res_rx.recv()).await;
    assert!(result.is_ok());
    
    // Clean shutdown
    drop(cmd_tx);
    handle.await.unwrap();
}
```

**Why channel-level:**
- Tests actual channel behavior (disconnect, backpressure if bounded)
- No abstraction overhead or complex mock setup
- Can verify exact message content sent
- Matches production code paths exactly

### Pattern 2: Mock Time for Timeout Testing

**What:** Use `tokio::time::pause()` and manual advancement to test timeout behavior without real delays.

**When to use:** For testing timeout handling, delays, and scheduling without slowing down tests.

**Example:**
```rust
// Source: Tokio docs on testing (https://tokio.rs/tokio/topics/testing)
#[tokio::test(start_paused = true)]
async fn test_operation_times_out() {
    tokio::time::pause();
    let start = tokio::time::Instant::now();
    
    // Setup channels
    let (tx, mut rx) = unbounded_channel::<EffectResult>();
    
    // Simulate slow background task that never sends
    tokio::spawn(async move {
        // Never sends anything - simulates stuck task
        tokio::time::sleep(Duration::from_secs(100)).await;
        let _ = tx.send(EffectResult::Success);
    });
    
    // Wait for result with timeout
    let result = tokio::time::timeout(
        Duration::from_secs(5),
        rx.recv()
    ).await;
    
    // Should timeout
    assert!(result.is_err(), "Should have timed out");
    
    // Verify only 5 seconds "passed"
    assert_eq!(start.elapsed(), Duration::from_secs(5));
}
```

**Key points:**
- `start_paused = true` pauses time at test start
- Time only advances when explicitly advanced or when no other work can proceed
- Tests run fast regardless of timeout duration being tested

### Pattern 3: Sequential Async Test Execution

**What:** Use `#[serial]` attribute from `serial_test` crate to prevent async tests from running in parallel.

**When to use:** When async tests share global state (e.g., static mocks, filesystem, or terminal settings).

**Example:**
```rust
// Source: serial_test crate documentation
use serial_test::serial;

#[tokio::test]
#[serial] // Prevents parallel execution with other serial tests
async fn test_shared_state_operation() {
    // This test has exclusive access to shared resources
    // Other serial tests will wait for this to complete
}

#[tokio::test]
#[serial]
async fn test_another_shared_operation() {
    // Runs after test_shared_state_operation completes
}

#[tokio::test]
async fn test_independent_operation() {
    // Can run in parallel with other non-serial tests
}
```

**Why sequential:**
- Async tests may share runtime state (e.g., tokio runtime configuration)
- Channel-based tests can interfere if using shared channel endpoints
- Prevents flaky tests due to timing/race conditions

### Pattern 4: Dual Assertion Strategy

**What:** Verify both (1) commands sent to mock match expected variants AND (2) final Model state updated correctly.

**When to use:** For comprehensive Elm Architecture tests where effects are triggered and state transitions must be verified.

**Example:**
```rust
// Pattern adapted from Elm Architecture testing
#[tokio::test]
#[serial]
async fn test_effect_triggers_command_and_updates_state() {
    // Setup: Create model in initial state
    let mut model = create_test_model();
    let artifact_index = 0;
    
    // Create channels to capture effect commands
    let (cmd_tx, mut cmd_rx) = unbounded_channel::<EffectCommand>();
    let (res_tx, res_rx) = unbounded_channel::<EffectResult>();
    
    // Step 1: Trigger effect via update function (pure)
    let msg = Msg::EnterPressed;
    let (new_model, effect) = update(model, msg);
    model = new_model;
    
    // Assert: Effect::RunGenerator was returned
    assert!(matches!(effect, Effect::RunGenerator { .. }));
    
    // Step 2: Convert effect to command (simulating runtime)
    let cmd = effect_to_command(&effect, artifact_index);
    
    // Assert: Command matches expected variant
    assert!(matches!(cmd, EffectCommand::RunGenerator { idx: 0, .. }));
    
    // Step 3: Send command via channel
    cmd_tx.send(cmd).unwrap();
    
    // Step 4: Simulate background processing
    tokio::spawn(async move {
        if let Some(cmd) = cmd_rx.recv().await {
            // Simulate success
            res_tx.send(EffectResult::GeneratorFinished {
                artifact_index: 0,
                success: true,
                error: None,
            }).unwrap();
        }
    });
    
    // Step 5: Receive result and update model
    let result = res_rx.recv().await.unwrap();
    let result_msg = Msg::from(result);
    let (final_model, _) = update(model, result_msg);
    
    // Assert: Final state is correct
    assert!(matches!(
        final_model.entries[0].status(),
        ArtifactStatus::Done { .. }
    ));
}
```

### Pattern 5: Testing tokio::select! Branches

**What:** Ensure each branch of a `tokio::select!` is covered by tests, including error paths and cancellation.

**When to use:** For the background task's main loop which uses `select!` to handle multiple async sources.

**Example:**
```rust
// Testing select! branches requires exercising each arm
#[tokio::test(start_paused = true)]
#[serial]
async fn test_select_shutdown_branch() {
    let (cmd_tx, cmd_rx) = unbounded_channel::<EffectCommand>();
    let (res_tx, mut res_rx) = unbounded_channel::<EffectResult>();
    let shutdown_token = CancellationToken::new();
    
    // Spawn background with select! loop
    let handle = tokio::spawn(async move {
        let mut shutdown = shutdown_token.child_token();
        let mut cmd_rx = cmd_rx;
        
        loop {
            tokio::select! {
                // Branch 1: Shutdown signal
                _ = shutdown.cancelled() => {
                    return "shutdown";
                }
                // Branch 2: Command received
                Some(cmd) = cmd_rx.recv() => {
                    // Process command
                }
                // Branch 3: Channel closed
                else => {
                    return "channel_closed";
                }
            }
        }
    });
    
    // Test shutdown branch by signalling cancellation
    shutdown_token.cancel();
    
    let result = tokio::time::timeout(Duration::from_secs(1), handle).await;
    assert!(result.unwrap().unwrap() == "shutdown");
}

#[tokio::test]
#[serial]
async fn test_select_channel_closed_branch() {
    // Drop sender to trigger "else" branch
    drop(cmd_tx);
    
    let result = handle.await.unwrap();
    assert!(result == "channel_closed");
}
```

**Coverage targets for select!:**
- Each branch arm executed at least once
- Branch preconditions (`if` clauses) tested
- Cancellation safety verified for each async operation

### Pattern 6: State Machine Simulation

**What:** Mock simulates complete lifecycle: Pending → Running → Success/Failed.

**When to use:** For testing the full artifact generation flow without actual script execution.

**Example:**
```rust
// Simulate full lifecycle through channel messages
#[tokio::test]
#[serial]
async fn test_full_lifecycle_state_machine() {
    let (cmd_tx, mut cmd_rx) = unbounded_channel::<EffectCommand>();
    let (res_tx, mut res_rx) = unbounded_channel::<EffectResult>();
    
    // Mock background that simulates state progression
    tokio::spawn(async move {
        while let Some(cmd) = cmd_rx.recv().await {
            match cmd {
                EffectCommand::CheckSerialization { artifact_index, .. } => {
                    // Simulate: Pending → Running → Success
                    res_tx.send(EffectResult::CheckFinished {
                        artifact_index,
                        needs_generation: true,
                    }).unwrap();
                }
                EffectCommand::RunGenerator { artifact_index, .. } => {
                    // Simulate generation delay
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    res_tx.send(EffectResult::GeneratorFinished {
                        artifact_index,
                        success: true,
                        error: None,
                    }).unwrap();
                }
                EffectCommand::Serialize { artifact_index, .. } => {
                    res_tx.send(EffectResult::SerializeFinished {
                        artifact_index,
                        success: true,
                        error: None,
                    }).unwrap();
                }
                _ => {}
            }
        }
    });
    
    // Drive the state machine through messages
    // Assert state transitions...
}
```

### Pattern 7: Controlled Async Delays

**What:** Use `tokio::time::sleep` in tests to simulate realistic async timing.

**When to use:** When testing order of operations or ensuring async operations complete in expected sequence.

**Example:**
```rust
#[tokio::test(start_paused = true)]
async fn test_async_ordering() {
    let (tx, mut rx) = unbounded_channel::<i32>();
    
    // Spawn multiple async operations with different delays
    let tx1 = tx.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        tx1.send(1).unwrap();
    });
    
    let tx2 = tx.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(10)).await;
        tx2.send(2).unwrap();
    });
    
    // With paused time, order depends on spawn order and sleep duration
    let first = rx.recv().await.unwrap();
    let second = rx.recv().await.unwrap();
    
    // Note: In real time, 2 arrives first (10ms < 50ms)
    // But with paused time, we may need to advance time manually
}
```

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Async test runtime | Manual runtime creation | `#[tokio::test]` | Simpler, standard, handles cleanup |
| Time mocking | Thread sleep in tests | `tokio::time::pause()` | Fast, deterministic, no real delays |
| Sequential async tests | Complex locking | `#[serial]` attribute | Simple, clear intent |
| Channel mocking | Complex wrapper types | Direct channel injection | Simpler, matches production |
| Mock time advancement | Thread::sleep | `tokio::time::advance()` | Works with paused time |
| Snapshot testing | Manual string asserts | `insta` crate | Already in use, better diffs |

**Key insight:** The project already uses `insta` for snapshot testing. Leverage it for async test output as well.

---

## Common Pitfalls

### Pitfall 1: Shared State Between Async Tests

**What goes wrong:** Tests interfere with each other due to shared global state (e.g., static variables, filesystem, terminal).

**Why it happens:** Async tests run concurrently by default; shared mutable state causes races.

**How to avoid:**
- Use `#[serial]` for tests that share state
- Use `tempfile::TempDir` for isolated filesystem operations
- Reset global state before each test

**Warning signs:** Flaky tests that pass individually but fail in CI or full runs.

### Pitfall 2: Not Using Paused Time

**What goes wrong:** Tests with timeouts take real time to run, slowing down the test suite.

**Why it happens:** Forgot to use `start_paused = true` or `tokio::time::pause()`.

**How to avoid:**
```rust
// ❌ Bad - takes 5 real seconds
#[tokio::test]
async fn test_timeout_slow() {
    let result = tokio::time::timeout(Duration::from_secs(5), async {
        // slow operation
    }).await;
}

// ✅ Good - completes instantly
#[tokio::test(start_paused = true)]
async fn test_timeout_fast() {
    let result = tokio::time::timeout(Duration::from_secs(5), async {
        // slow operation
    }).await;
}
```

### Pitfall 3: Testing With Unbounded Channels Only

**What goes wrong:** Tests pass with unbounded channels but fail with bounded channels (or vice versa).

**Why it happens:** Different behavior on send when channel is full.

**How to avoid:** Test with the same channel type used in production (project uses unbounded, so this is fine).

### Pitfall 4: Not Testing Channel Disconnect

**What goes wrong:** Application panics or hangs when channel disconnects because it wasn't tested.

**Why it happens:** Happy path testing only; error cases neglected.

**How to avoid:** Explicitly test disconnect scenarios:
```rust
#[tokio::test]
async fn test_graceful_disconnect() {
    let (tx, mut rx) = unbounded_channel::<Msg>();
    
    // Simulate TUI closing channel
    drop(tx);
    
    // Background should handle this gracefully
    let result = rx.recv().await;
    assert!(result.is_none()); // Channel closed
}
```

### Pitfall 5: Deadlocks in Test Setup

**What goes wrong:** Test deadlocks because it's waiting for a message that never comes.

**Why it happens:** Forgot to spawn the task that sends the message, or order of operations wrong.

**How to avoid:** Always use `tokio::time::timeout` in tests to prevent infinite waits:
```rust
// ❌ Bad - can deadlock forever
let msg = rx.recv().await.unwrap();

// ✅ Good - fails fast if no message
let msg = tokio::time::timeout(Duration::from_secs(1), rx.recv())
    .await
    .expect("timeout waiting for message")
    .expect("channel closed");
```

### Pitfall 6: Not Cleaning Up Tasks

**What goes wrong:** Test tasks continue running after test completes, causing interference.

**Why it happens:** Spawned tasks not awaited or cancelled.

**How to avoid:**
```rust
#[tokio::test]
async fn test_with_cleanup() {
    let handle = tokio::spawn(async {
        // background work
    });
    
    // ... test code ...
    
    // Ensure cleanup
    drop(tx); // Signal shutdown
    let _ = tokio::time::timeout(Duration::from_secs(1), handle).await;
}
```

### Pitfall 7: Ignoring Cancellation Safety

**What goes wrong:** Operations in `select!` branches lose data when cancelled.

**Why it happens:** Using non-cancellation-safe operations in select! branches.

**How to avoid:** Use only cancellation-safe operations:
- ✅ `recv()` on mpsc channels - safe
- ✅ `recv()` on oneshot channels - safe  
- ❌ `read_exact()` on I/O - not safe (use read + track progress)

See tokio docs for full list of cancellation-safe operations.

---

## Code Examples

### Complete Async Test Example

```rust
// tests/async/background_tests.rs
use tokio::sync::mpsc::unbounded_channel;
use tokio::time::{Duration, pause};
use serial_test::serial;

#[tokio::test(start_paused = true)]
#[serial]
async fn test_background_job_executes_effect() {
    // Setup channels
    let (cmd_tx, cmd_rx) = unbounded_channel::<EffectCommand>();
    let (res_tx, mut res_rx) = unbounded_channel::<EffectResult>();
    
    // Create background handler
    let handler = BackgroundEffectHandler::new(
        create_test_backend(),
        create_test_make()
    );
    
    // Spawn background task
    let mut bg = BackgroundTask::new(handler, cmd_rx, res_tx);
    let handle = tokio::spawn(async move {
        bg.run().await
    });
    
    // Send command
    cmd_tx.send(EffectCommand::CheckSerialization {
        artifact_index: 0,
        artifact_name: "test".to_string(),
    }).unwrap();
    
    // Receive result with timeout
    let result = tokio::time::timeout(
        Duration::from_secs(1),
        res_rx.recv()
    ).await
    .expect("timeout")
    .expect("channel closed");
    
    // Assert
    assert!(matches!(result, EffectResult::CheckFinished { .. }));
    
    // Cleanup
    drop(cmd_tx);
    let _ = handle.await;
}
```

### Integration Test with Snapshot

```rust
// tests/cli/tui_tests.rs - No changes needed for async
// Integration tests verify CLI interface, not async internals

use insta_cmd::{assert_cmd_snapshot, get_cargo_bin};
use std::process::Command;

#[test]
#[serial]
fn test_tui_list_artifacts() {
    // This test uses real subprocess - async is internal
    // Snapshot verifies output is correct
    assert_cmd_snapshot!(
        Command::new(get_cargo_bin("artifacts"))
            .arg("tui")
            .arg("--machine")
            .arg("test-machine")
            .current_dir("examples/scenarios/single-artifact")
    );
}
```

### Channel Disconnect Test

```rust
#[tokio::test]
#[serial]
async fn test_channel_disconnect_handling() {
    let (cmd_tx, cmd_rx) = unbounded_channel::<EffectCommand>();
    let (res_tx, mut res_rx) = unbounded_channel::<EffectResult>();
    
    // Spawn background
    let handle = tokio::spawn(async move {
        let mut handler = create_test_handler();
        
        loop {
            tokio::select! {
                Some(cmd) = cmd_rx.recv() => {
                    if res_tx.send(handler.execute(cmd).await).is_err() {
                        // TUI closed - exit gracefully
                        break;
                    }
                }
                else => break, // Channel closed
            }
        }
        
        "exited_gracefully"
    });
    
    // Simulate TUI disconnect
    drop(cmd_tx);
    
    // Background should exit gracefully
    let result = tokio::time::timeout(Duration::from_secs(1), handle)
        .await
        .expect("timeout")
        .expect("task panicked");
    
    assert_eq!(result, "exited_gracefully");
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|-------------|------------------|--------------|--------|
| Manual test runtime | `#[tokio::test]` | 2019+ | Simpler, handles cleanup |
| Real time in tests | `tokio::time::pause()` | 2020+ | Fast, deterministic |
| Sleep-based ordering | Explicit synchronization | 2020+ | More reliable |
| Global test state | Isolated test fixtures | 2019+ | Parallel safe |
| Mock external services | Channel-level mocks | 2024 | Closer to production |

---

## Open Questions

1. **How to test shutdown with CancellationToken?**
   - What we know: Use `CancellationToken::new()` and `child_token()`
   - What's unclear: Best way to verify background task respected cancellation
   - Recommendation: Test that background exits after processing in-flight command

2. **Should async tests be in separate crate?**
   - What we know: Project has `tests/` directory for integration tests
   - What's unclear: Whether async unit tests belong in `tests/async/` or inline
   - Recommendation: Inline `#[cfg(test)]` for private functions, `tests/async/` for integration-style async tests

3. **How to achieve 80% coverage on select! branches?**
   - What we know: Each branch needs test coverage
   - What's unclear: How to measure branch coverage specifically
   - Recommendation: Use `cargo tarpaulin` or `cargo llvm-cov` for coverage metrics

---

## Sources

### Primary (HIGH confidence)

- Tokio testing docs: https://tokio.rs/tokio/topics/testing
- Tokio graceful shutdown: https://tokio.rs/tokio/topics/shutdown
- tokio::select! macro docs: https://docs.rs/tokio/latest/tokio/macro.select.html
- tokio::sync::mpsc docs: https://docs.rs/tokio/latest/tokio/sync/mpsc/
- tokio-test crate docs: https://docs.rs/tokio-test/0.4/tokio_test/

### Secondary (MEDIUM confidence)

- Alice Ryhl's blog on async blocking: https://ryhl.io/blog/async-what-is-blocking/
- Rust Async Book: https://rust-lang.github.io/async-book/
- Mini-redis graceful shutdown example: https://github.com/tokio-rs/mini-redis

### Tertiary (LOW confidence)

- Community testing patterns (2024-2025)
- Various GitHub issues on tokio testing

---

## Metadata

**Confidence breakdown:**

- Async testing patterns: HIGH - Well-established tokio patterns
- Channel mocking: HIGH - Standard approach, matches tokio examples
- Elm Architecture with async: MEDIUM-HIGH - Adapted from sync patterns
- Coverage targets: MEDIUM - Depends on tooling

**Research date:** 2026-02-14  
**Valid until:** 2026-03-14 (30 days for stable tokio ecosystem)

---

## User Constraints (from CONTEXT.md)

### Locked Decisions

- Channel-level mocks for unit tests (not handler-level abstractions)
- No mocks for integration tests (real background job)
- Sequential execution with `#[serial]`
- Dedicated `tests/async/` directory for async tests
- 80% minimum coverage for async channel components
- Mock time with `tokio::time::pause()` for timing tests

### Claude's Discretion

- Exact test declaration style (though `#[tokio::test]` is standard)
- Which existing integration tests need updating
- Specific organization within `tests/async/` directory
- Coverage tooling choice

### Deferred Ideas (OUT OF SCOPE)

- Property-based testing for async code
- Fuzz testing for channel protocols
- Load testing for background job
