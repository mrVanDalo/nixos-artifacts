# Phase 4: Robustness — Error Handling & Shutdown - Research

**Researched:** 2025-02-14  
**Domain:** Rust async error handling, tokio shutdown patterns, TUI cleanup  
**Confidence:** HIGH

## Summary

Phase 4 implements comprehensive error handling and clean shutdown behavior for a Rust/TUI application using tokio and ratatui. The project already has a solid foundation with:

- **Channel-based communication**: `tokio::sync::mpsc::unbounded_channel` between foreground (TUI) and background task
- **EffectResult error encoding**: Bool+Option<String> pattern for error propagation (e.g., `GeneratorFinished { success: bool, error: Option<String> }`)
- **Timeout protection**: All blocking I/O uses `tokio::time::timeout` around `spawn_blocking`
- **Panic hook**: Terminal restoration on panic via `install_panic_hook()`

The research focuses on four areas needed to fulfill requirements:

1. **Script error propagation** (ERRH-01): Already partially implemented via EffectResult - needs TUI display integration
2. **Graceful shutdown** (SHUT-01/02): Requires shutdown signal coordination between TUI and background task
3. **Background panic isolation** (ERRH-03): Needs `catch_unwind` or `JoinSet` to isolate panics
4. **Cleanup guarantee** (SHUT-03): Temp directories already use RAII (`tempfile::TempDir`), but shutdown must ensure handler is dropped

**Primary recommendation:** Implement shutdown using `tokio_util::sync::CancellationToken` for cooperative cancellation, wrap background execution in `catch_unwind` for panic isolation, and ensure proper drop order in shutdown sequence.

---

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| tokio | 1.x | Async runtime, channels, spawn_blocking | Already in use, standard for Rust async |
| tokio-util | 0.7.x | CancellationToken, additional utilities | Standard extension for tokio |
| futures | 0.3.x | Future combinators, channel utilities | Ecosystem standard |
| tempfile | 3.x | Temporary directory management | Already in use, RAII cleanup |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| std::panic::catch_unwind | std | Isolate panics in spawn_blocking | For ERRH-03 panic isolation |
| anyhow | 1.x | Error handling | Already in use |
| thiserror | 1.x | Structured error types | If adding error variants |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| CancellationToken | AbortHandle + JoinSet | JoinSet requires more refactoring, CancellationToken is more composable |
| catch_unwind | JoinSet + panic handling | JoinSet doesn't catch spawn_blocking panics automatically |
| Unbounded channels | Bounded channels | Bounded adds backpressure complexity; unbounded fits current design |

**Installation:** Already present in Cargo.toml - tokio-util needs feature flag:
```toml
tokio-util = { version = "0.7", features = ["full"] }
```

---

## Architecture Patterns

### Recommended Project Structure

Current structure already supports the patterns:

```
pkgs/artifacts/src/
├── tui/
│   ├── runtime.rs      # Main loop, shutdown coordination
│   ├── background.rs   # Background task with panic isolation
│   ├── channels.rs     # EffectCommand/EffectResult definitions
│   └── terminal.rs     # TerminalGuard with RAII cleanup
├── app/
│   ├── model.rs        # Model with error field
│   ├── message.rs      # Msg variants for errors
│   └── update.rs       # State transitions on errors
└── backend/
    └── temp_dir.rs     # RAII temp directory management
```

### Pattern 1: Cooperative Cancellation with CancellationToken

**What:** Token-based cancellation that allows background tasks to check for shutdown requests and complete gracefully.

**When to use:** For SHUT-01 (clean shutdown) and SHUT-02 (complete or cancel in-flight effects).

**Example:**
```rust
// Source: tokio-util docs + standard patterns
use tokio_util::sync::CancellationToken;

pub fn spawn_background_task(
    backend: BackendConfiguration,
    make: MakeConfiguration,
    shutdown_token: CancellationToken,  // NEW
) -> (UnboundedSender<EffectCommand>, UnboundedReceiver<EffectResult>) {
    let (tx_cmd, mut rx_cmd) = unbounded_channel::<EffectCommand>();
    let (tx_res, rx_res) = unbounded_channel::<EffectResult>();

    tokio::spawn(async move {
        let mut handler = BackgroundEffectHandler::new(backend, make);
        
        loop {
            tokio::select! {
                // Check for shutdown signal
                _ = shutdown_token.cancelled() => {
                    log_component("BACKGROUND", "Shutdown requested, exiting");
                    // Handler dropped here, cleaning up temp dirs
                    break;
                }
                
                // Process next command or exit if channel closed
                Some(cmd) = rx_cmd.recv() => {
                    let result = handler.execute(cmd).await;
                    if tx_res.send(result).is_err() {
                        break; // TUI closed
                    }
                }
                
                // Channel closed, exit naturally
                else => break,
            }
        }
    });

    (tx_cmd, rx_res)
}
```

### Pattern 2: Panic Isolation with catch_unwind

**What:** Wrap spawn_blocking closures in `catch_unwind` to prevent task panics from crashing the entire application.

**When to use:** For ERRH-03 (background panics don't crash TUI).

**Example:**
```rust
// Source: Standard library panic handling patterns
use std::panic::AssertUnwindSafe;

// In BackgroundEffectHandler::execute()
let result = timeout(
    BACKGROUND_TASK_TIMEOUT,
    tokio::task::spawn_blocking(move || {
        // Wrap the operation in catch_unwind
        match std::panic::catch_unwind(AssertUnwindSafe(|| {
            backend::generator::run_generator_script(...)
        })) {
            Ok(Ok(output)) => Ok(output),           // Success
            Ok(Err(e)) => Err(e),                 // Business error
            Err(_) => Err(anyhow!("Task panicked")), // Panic caught
        }
    })
).await;
```

**Key considerations:**
- `AssertUnwindSafe` is safe here because the closure doesn't share mutable state across unwind boundary
- Must avoid types that aren't unwind-safe (e.g., MutexGuard across panic)
- Current code already has timeout + spawn_blocking; just add catch_unwind layer

### Pattern 3: Graceful Shutdown Sequence

**What:** Structured shutdown that: 1) signals background task, 2) waits for in-flight work, 3) cleans up resources.

**When to use:** For SHUT-01, SHUT-02, SHUT-03.

**Flow:**
```
User presses 'q' or SIGINT
         │
         ▼
┌──────────────────────┐
│ 1. Set quit flag     │
│    (stop new work)   │
└──────────────────────┘
         │
         ▼
┌──────────────────────┐
│ 2. Cancel token      │
│    (signal shutdown) │
└──────────────────────┘
         │
         ▼
┌──────────────────────┐
│ 3. Drain commands    │
│    (finish in-flight)│
└──────────────────────┘
         │
         ▼
┌──────────────────────┐
│ 4. Drop channels     │
│    (background exits)│
└──────────────────────┘
         │
         ▼
┌──────────────────────┐
│ 5. Drop handler      │
│    (temp dirs clean) │
└──────────────────────┘
         │
         ▼
┌──────────────────────┐
│ 6. Restore terminal  │
└──────────────────────┘
```

**Implementation:**
```rust
// In runtime.rs::run_async() shutdown path
if effect.is_quit() {
    // 1. Signal background task to shut down
    shutdown_token.cancel();
    
    // 2. Drain any pending results (graceful completion)
    loop {
        match res_rx.try_recv() {
            Ok(result) => {
                // Process final results
                let msg = result_to_message(result);
                let (new_model, _) = update(model, msg);
                model = new_model;
            }
            Err(_) => break, // No more results
        }
    }
    
    // 3. Drop command channel (causes background to exit after current work)
    drop(cmd_tx);
    
    // 4. Channels dropped naturally at end of scope
    // 5. Handler dropped, temp dirs cleaned up
    break;
}
```

### Pattern 4: Error Display in TUI

**What:** Convert effect execution failures (exit code, output) into user-visible error state.

**When to use:** For ERRH-04 (effect failures show in TUI).

**Current state:** `ArtifactStatus::Failed` already exists in model.rs:
```rust
pub enum ArtifactStatus {
    Failed {
        error: String,
        output: String,
        retry_available: bool,
    },
    // ...
}
```

**Usage in update.rs:**
```rust
// When receiving error result
Msg::GeneratorFinished { artifact_index, result: Err(error), .. } => {
    let entry = &mut model.entries[artifact_index];
    *entry.status_mut() = ArtifactStatus::Failed {
        error: error.clone(),
        output: format!("Exit code: {}\nOutput:\n{}", exit_code, captured_output),
        retry_available: true,
    };
    // ...
}
```

### Pattern 5: Channel Disconnect Detection

**What:** Detect when channel is disconnected and handle gracefully without panic.

**When to use:** For ERRH-02 (graceful disconnect handling).

**Current implementation already handles this:**
```rust
// In background.rs
while let Some(cmd) = rx_cmd.recv().await {
    // ...
    if tx_res.send(result).is_err() {
        log_component("BACKGROUND", "TUI closed (channel closed), exiting");
        break; // Graceful exit, not panic
    }
}
```

**Additional protection in foreground:**
```rust
// In runtime.rs::run_async()
if cmd_tx.send(cmd).is_err() {
    log_component("RUNTIME", "Background task closed, exiting");
    return Ok(RunResult { ... });
}
```

### Anti-Patterns to Avoid

- **Aborting tasks abruptly:** Don't use `task.abort()` directly without cleanup; use CancellationToken
- **Ignoring spawn_blocking panics:** Must use catch_unwind to prevent propagation
- **Long-running without cancellation checks:** Effects should periodically check token (though current sequential design is fine)
- **Manual temp dir cleanup:** Don't use `std::fs::remove_dir_all` - rely on `TempDir` RAII
- **Blocking in async context:** Never do blocking I/O without spawn_blocking (already correct)

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Shutdown signaling | Manual atomic bool + sleep | `tokio_util::sync::CancellationToken` | Proper async integration, composable |
| Temp directory cleanup | Manual rm -rf on paths | `tempfile::TempDir` RAII | Atomic, handles edge cases, already in use |
| Terminal restoration | Manual disable_raw_mode | `TerminalGuard` Drop impl | RAII guarantees cleanup even on panic |
| Panic isolation | Task per effect | `catch_unwind` in spawn_blocking | Standard pattern for spawn_blocking |
| Channel disconnect | Panic on send error | `is_err()` check + graceful exit | Standard mpsc behavior |

**Key insight:** The project already uses several of these (tempfile, TerminalGuard). The additions are CancellationToken for shutdown and catch_unwind for panic isolation.

---

## Common Pitfalls

### Pitfall 1: Not Catching Panics in spawn_blocking

**What goes wrong:** A panic in the generator or serialize script crashes the entire background task, which propagates and may crash TUI.

**Why it happens:** `tokio::task::spawn_blocking` panics propagate to the JoinHandle; if not handled, they crash the task.

**How to avoid:** Wrap all spawn_blocking operations in `catch_unwind`:
```rust
tokio::task::spawn_blocking(move || {
    std::panic::catch_unwind(AssertUnwindSafe(|| {
        // Actual work
    })).map_err(|_| anyhow!("Task panicked"))
})
```

**Warning signs:** Tests with deliberately panicking generators cause TUI tests to fail completely.

### Pitfall 2: Temp Directory Not Cleaned on Early Exit

**What goes wrong:** If the background task is aborted abruptly (abort() instead of graceful exit), `current_output_dir` in handler may not be dropped, leaking temp directories.

**Why it happens:** `TempDir` only cleans on drop; abort() doesn't run destructors.

**How to avoid:** Use graceful shutdown (CancellationToken + drop) instead of `task.abort()`.

**Warning signs:** `/tmp` directory accumulates `.tmpXXXXXX` directories after runs.

### Pitfall 3: Terminal Not Restored on Signal

**What goes wrong:** User presses Ctrl+C, process exits, but terminal remains in raw mode / alternate screen.

**Why it happens:** Signal handler doesn't restore terminal before exit.

**How to avoid:** Use `tokio::signal::ctrl_c()` to intercept signal, then run shutdown sequence which includes terminal restoration.

**Warning signs:** After Ctrl+C, terminal shows garbled output, arrow keys don't work.

### Pitfall 4: Background Task Hangs on Shutdown

**What goes wrong:** Shutdown signal sent but background task doesn't exit because it's blocked on a long-running script.

**Why it happens:** No timeout on graceful shutdown wait; script ignores signals.

**How to avoid:** Have a two-phase shutdown: 1) Try graceful with timeout (e.g., 5s), 2) If still running, drop channels to force exit. The existing timeout on operations handles this.

**Warning signs:** TUI closes but process doesn't exit for minutes.

### Pitfall 5: Race Between Result Send and Shutdown

**What goes wrong:** Background task sends result, shutdown occurs, result is lost, user sees "in progress" forever.

**Why it happens:** Foreground stops processing results before draining channel.

**How to avoid:** Always drain result channel completely before exiting, even after shutdown signal.

**Warning signs:** Artifacts stuck in "Generating" status when quitting.

---

## Code Examples

### Complete Background Task with All Patterns

```rust
// Source: Adapted from existing background.rs + standard patterns
use tokio_util::sync::CancellationToken;
use std::panic::AssertUnwindSafe;

pub fn spawn_background_task(
    backend: BackendConfiguration,
    make: MakeConfiguration,
    shutdown_token: CancellationToken,
) -> (UnboundedSender<EffectCommand>, UnboundedReceiver<EffectResult>) {
    let (tx_cmd, mut rx_cmd) = unbounded_channel::<EffectCommand>();
    let (tx_res, rx_res) = unbounded_channel::<EffectResult>();

    tokio::spawn(async move {
        let mut handler = BackgroundEffectHandler::new(backend, make);
        
        loop {
            tokio::select! {
                // Shutdown signal received
                _ = shutdown_token.cancelled() => {
                    log_component("BACKGROUND", "Shutdown requested");
                    // Process remaining commands then exit
                    while let Ok(cmd) = rx_cmd.try_recv() {
                        let result = handler.execute(cmd).await;
                        let _ = tx_res.send(result); // Best effort
                    }
                    break;
                }
                
                // Normal operation
                Some(cmd) = rx_cmd.recv() => {
                    // Execute with panic isolation
                    let result = handler.execute_with_isolation(cmd).await;
                    if tx_res.send(result).is_err() {
                        break; // TUI closed
                    }
                }
                
                else => break, // Channel closed
            }
        }
        
        log_component("BACKGROUND", "Exiting, handler will be dropped (temp dirs cleaned)");
    });

    (tx_cmd, rx_res)
}

// In BackgroundEffectHandler
async fn execute_with_isolation(&mut self, cmd: EffectCommand) -> EffectResult {
    // Wrap execution in catch_unwind
    match std::panic::catch_unwind(AssertUnwindSafe(|| {
        // Can't run async inside catch_unwind, so use block_on or restructure
        // Better: catch_unwind inside the spawn_blocking
    })) {
        Ok(result) => result,
        Err(_) => EffectResult::Error {
            error: "Task panicked".to_string(),
        },
    }
}
```

### Shutdown with Signal Handling

```rust
// Source: Standard tokio signal handling patterns
use tokio::signal;

pub async fn run_async<B, E>(...) -> Result<RunResult> {
    // Create shutdown token
    let shutdown_token = CancellationToken::new();
    let child_token = shutdown_token.child_token();
    
    // Spawn background with token
    let (cmd_tx, mut res_rx) = spawn_background_task(backend, make, child_token);
    
    // Setup Ctrl+C handler
    let shutdown_for_signal = shutdown_token.clone();
    tokio::spawn(async move {
        if let Ok(()) = signal::ctrl_c().await {
            log_component("RUNTIME", "Ctrl+C received, requesting shutdown");
            shutdown_for_signal.cancel();
        }
    });
    
    loop {
        // Check for shutdown in addition to quit
        if shutdown_token.is_cancelled() && !shutting_down {
            shutting_down = true;
            // Initiate graceful shutdown
        }
        
        // ... rest of loop
        
        if effect.is_quit() || shutdown_token.is_cancelled() {
            // Draining sequence...
            break;
        }
    }
    
    // Ensure cleanup
    drop(cmd_tx);
    // res_rx dropped at end of scope
    Ok(...)
}
```

### Error Display Integration

```rust
// In update.rs - already partially exists
fn update(model: Model, msg: Msg) -> (Model, Effect) {
    match (model.screen, msg) {
        // ... other handlers ...
        
        (_, Msg::GeneratorFinished { artifact_index, result: Err(error), .. }) => {
            let mut new_model = model.clone();
            if let Some(entry) = new_model.entries.get_mut(artifact_index) {
                entry.status = ArtifactStatus::Failed {
                    error: error.clone(),
                    output: String::new(), // Could populate from result if available
                    retry_available: true,
                };
            }
            (new_model, Effect::None)
        }
        
        // Similar for other effect results
        _ => (model, Effect::None),
    }
}
```

---

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|-------------|------------------|--------------|---------|
| Manual task abort | CancellationToken | 2020+ | Graceful, composable cancellation |
| Panic = crash | catch_unwind isolation | Always standard | Resilience |
| Bounded channels | Unbounded channels | Project decision | Simpler, no backpressure |
| Explicit cleanup | RAII (TempDir, TerminalGuard) | Already adopted | Guaranteed cleanup |
| Synchronous effects | spawn_blocking + timeout | Already adopted | Non-blocking, protected |

**Deprecated/outdated:**

- `tokio::task::abort()` as primary shutdown: Use CancellationToken instead
- Manual temp file cleanup: Use `tempfile` crate RAII
- `std::panic::resume_unwind`: Not needed with proper catch_unwind

---

## Open Questions

1. **Should in-flight effects complete or cancel immediately on shutdown?**
   - What we know: SHUT-02 says "complete before shutdown (or cancel gracefully)"
   - What's unclear: Which takes priority - user wants fast exit, or wants work saved?
   - Recommendation: Complete current effect, don't start new ones, 5s timeout then force exit

2. **How to display multi-target shared artifact errors?**
   - What we know: Shared artifacts have multiple targets, each can fail
   - What's unclear: Should TUI show per-target errors or aggregate?
   - Recommendation: Show aggregate in list, per-target in log view (already have step_logs)

3. **Should retry be implemented for transient failures?**
   - What we know: `retry_available: bool` exists in Failed status
   - What's unclear: Is retry in scope for this phase or later?
   - Recommendation: Keep retry_available flag but implement actual retry in Phase 5

---

## Sources

### Primary (HIGH confidence)

- `pkgs/artifacts/src/tui/background.rs` - Current background task implementation
- `pkgs/artifacts/src/tui/channels.rs` - Channel message definitions
- `pkgs/artifacts/src/tui/runtime.rs` - Main TUI loop with async support
- `pkgs/artifacts/src/app/model.rs` - Model with ArtifactStatus::Failed
- `pkgs/artifacts/src/tui/terminal.rs` - TerminalGuard RAII cleanup
- tokio-util 0.7 docs (CancellationToken): https://docs.rs/tokio-util/latest/tokio_util/sync/cancellation_token/
- std::panic docs: https://doc.rust-lang.org/std/panic/

### Secondary (MEDIUM confidence)

- Rust Async Book - Graceful Shutdown patterns
- Tokio docs - Signal handling: https://docs.rs/tokio/latest/tokio/signal/
- tempfile docs - RAII cleanup behavior

### Tertiary (LOW confidence)

- Community blog posts on tokio shutdown (2024-2025)
- Various GitHub issues on graceful shutdown

---

## Metadata

**Confidence breakdown:**

- Standard stack: HIGH - tokio-util CancellationToken is well-established, catch_unwind is standard
- Architecture: HIGH - Patterns align with existing codebase structure
- Pitfalls: MEDIUM-HIGH - Based on common Rust async patterns and specific codebase analysis

**Research date:** 2025-02-14  
**Valid until:** 2025-03-14 (30 days for stable tokio ecosystem)

---

## User Constraints (from phase context)

### Locked Decisions

- Sequential processing of effects (FIFO queue) - Must maintain existing ordering
- Unbounded channels (no backpressure) - Keep current mpsc::unbounded_channel
- artifact_index in every message enables dispatch - Preserve in all EffectResult variants
- Errors in result messages use bool+Option<String> pattern - Continue existing encoding
- Handler owns config, no shared state - Keep BackgroundEffectHandler ownership model
- Timeout-based event polling with 50ms timeout - Current runtime.rs behavior
- spawn_blocking for all blocking I/O - Already implemented, preserve

### Claude's Discretion

- Shutdown signaling mechanism (CancellationToken vs alternatives)
- Panic isolation strategy (catch_unwind vs JoinSet)
- Error display UI specifics (how to show exit code and output)
- Graceful shutdown timeout duration (configurable?)
- Signal handling integration (Ctrl+C behavior)

### Deferred Ideas (OUT OF SCOPE)

- Retry mechanism for transient failures (flag exists but implementation deferred)
- Bounded channels with backpressure (explicitly not in this phase)
- Streaming output from scripts (complete output returned at end per CONTEXT)
