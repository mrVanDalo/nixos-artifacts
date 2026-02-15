# Phase 1: Foundation — Core Architecture - Research

**Researched:** 2025-02-13 **Domain:** Rust TUI async architecture with tokio
channels **Confidence:** HIGH

## Summary

This phase establishes the foundational async communication architecture for the
Artifacts TUI. The current implementation uses a synchronous effect handler
pattern that blocks the TUI during long-running operations like generator
scripts. The refactor will introduce tokio-based background task execution with
mpsc channels to enable non-blocking operation.

Based on analysis of the existing codebase (ratatui 0.29, crossterm 0.28, tokio
available), the architecture requires:

1. **Two-way channel communication**: TUI foreground sends `Effect` messages to
   a background task via one channel; background sends `EffectResult` messages
   back via another.

2. **Sequential FIFO processing**: Effects must execute in order to maintain
   data dependencies (e.g., RunGenerator must complete before Serialize can
   access the output directory).

3. **State isolation**: Model remains in the foreground thread only; background
   receives immutable data snapshots needed for execution.

4. **Clean runtime loop**: The main TUI loop (`run()` in runtime.rs) currently
   blocks in `execute_effect()` calls. It must be refactored to poll both
   terminal events AND channel messages concurrently.

**Primary recommendation**: Use `tokio::sync::mpsc::unbounded_channel` for
unbounded buffering, with a single background task spawned at startup. This
follows the standard Rust async pattern used in ratatui applications that
require async work.

## User Constraints (from CONTEXT.md)

### Locked Decisions

- **Message types:** Separate per-effect variants — `CheckSerialization`,
  `GeneratorFinished`, `SerializeFinished`, `SharedCheckSerialization`,
  `SharedGeneratorFinished`, `SharedSerializeFinished`
- **Channel capacity:** Unbounded (no backpressure, TUI never blocks)
- **Message content:** Include artifact ID in every message for dispatch context
- **Script output:** Complete output returned at end of execution (buffered, not
  streamed)
- **Directionality:**
  - Foreground → Background: `Effect` messages (what to execute)
  - Background → Foreground: `EffectResult` messages (outcomes)
  - Sequential processing: Effects execute FIFO in single background task
- **Error Handling:** Errors travel in result messages, not separate error
  channel

### Claude's Discretion

- **Result structure:** Follow Rust/Ratatui conventions
- **Error variant design:** Specific error variants at Claude's discretion

### Deferred Ideas (OUT OF SCOPE)

- None — discussion stayed within phase scope

## Standard Stack

### Core

| Library           | Version | Purpose                         | Why Standard                                            |
| ----------------- | ------- | ------------------------------- | ------------------------------------------------------- |
| tokio             | 1.42+   | Async runtime and mpsc channels | Already used by ratatui 0.29 for internal async support |
| tokio::sync::mpsc | bundled | Channel communication           | Standard Rust async channel (docs.rs verified)          |
| tokio::task       | bundled | Background task spawn           | Standard for spawning async tasks                       |
| anyhow            | 1.x     | Error propagation               | Already used throughout codebase                        |

### Supporting

| Library        | Version | Purpose                  | When to Use                                              |
| -------------- | ------- | ------------------------ | -------------------------------------------------------- |
| std::future    | stdlib  | Future trait             | For async trait bounds                                   |
| tokio::select! | bundled | Concurrent event polling | Main loop needs to poll both events and channel messages |

### Alternatives Considered

| Instead of             | Could Use         | Tradeoff                                                                              |
| ---------------------- | ----------------- | ------------------------------------------------------------------------------------- |
| `tokio::sync::mpsc`    | `std::sync::mpsc` | std mpsc is blocking; would require separate thread per effect                        |
| `unbounded_channel`    | `channel(n)`      | Unbounded = no TUI blocking at cost of memory; bounded requires backpressure handling |
| Single background task | Task per effect   | Multiple tasks complicate FIFO ordering and require extra synchronization             |

## Architecture Patterns

### Recommended Project Structure

```
src/
├── tui/
│   ├── runtime.rs           # Modified: Main loop with tokio integration
│   ├── effect_handler.rs    # Replaced: Old synchronous handler
│   ├── background.rs          # NEW: Background task implementation
│   └── channels.rs            # NEW: Channel message types
├── app/
│   ├── effect.rs            # Modified: Effect variants for async dispatch
│   ├── message.rs           # Modified: Add EffectResult variants
│   └── ...
└── backend/
    └── ...                  # (Unchanged: serialization.rs, generator.rs)
```

### Pattern 1: Channel-Based Effect Dispatch

**What:** TUI sends Effect messages to a background task that executes them
asynchronously.

**When to use:** When effects take significant time and TUI must remain
responsive.

**Example:**

```rust
// Source: Based on tokio docs (docs.rs/tokio/1.42.0/tokio/sync/mpsc/)
// and current codebase architecture

// Channel message types (src/tui/channels.rs)
pub enum EffectCommand {
    CheckSerialization { artifact_index: usize, /* ... */ },
    RunGenerator { artifact_index: usize, /* ... */ },
    Serialize { artifact_index: usize, /* ... */ },
    // ... shared variants
}

pub enum EffectResult {
    CheckSerialization { artifact_index: usize, result: Result<bool, String>, output: Option<CheckOutput> },
    GeneratorFinished { artifact_index: usize, result: Result<GeneratorOutput, String> },
    SerializeFinished { artifact_index: usize, result: Result<SerializeOutput, String> },
    // ... shared variants
}

// Background task (src/tui/background.rs)
pub fn spawn_background_task(
    backend: BackendConfiguration,
    make: MakeConfiguration,
) -> (UnboundedSender<EffectCommand>, UnboundedReceiver<EffectResult>) {
    let (tx_cmd, mut rx_cmd) = mpsc::unbounded_channel();
    let (tx_res, rx_res) = mpsc::unbounded_channel();
    
    tokio::spawn(async move {
        while let Some(cmd) = rx_cmd.recv().await {
            let result = execute_effect(cmd, &backend, &make).await;
            if tx_res.send(result).is_err() {
                break; // TUI closed
            }
        }
    });
    
    (tx_cmd, rx_res)
}

// Main runtime loop modification (src/tui/runtime.rs)
pub async fn run_async<B>(
    terminal: &mut Terminal<B>,
    events: &mut impl EventSource,
    cmd_tx: UnboundedSender<EffectCommand>,
    mut res_rx: UnboundedReceiver<EffectResult>,
    mut model: Model,
) -> Result<RunResult>
where
    B: Backend,
{
    let mut frames_rendered = 0;
    
    loop {
        terminal.draw(|f| render(f, &model))?;
        frames_rendered += 1;
        
        tokio::select! {
            // Terminal events
            msg = events.next_event_async() => {
                if let Some(msg) = msg {
                    let (new_model, effect) = update(model, msg);
                    model = new_model;
                    // Send effect to background task
                    if let Some(cmd) = effect_to_command(effect) {
                        let _ = cmd_tx.send(cmd);
                    }
                }
            }
            // Effect results from background
            Some(result) = res_rx.recv() => {
                let msg = result_to_message(result);
                let (new_model, _) = update(model, msg);
                model = new_model;
            }
        }
    }
}
```

### Pattern 2: Effect-to-Command Mapping

**What:** Convert `Effect` enum variants into `EffectCommand` for channel
transmission.

**When to use:** When you need to separate TUI-facing API from background task
API.

**Example:**

```rust
// Convert Effect to channel command
fn effect_to_command(effect: Effect) -> Option<EffectCommand> {
    match effect {
        Effect::CheckSerialization { artifact_index, artifact_name, target, target_type } => {
            Some(EffectCommand::CheckSerialization { artifact_index, artifact_name, target, target_type })
        }
        Effect::RunGenerator { artifact_index, artifact_name, target, target_type, prompts } => {
            Some(EffectCommand::RunGenerator { artifact_index, artifact_name, target, target_type, prompts })
        }
        // ... other variants
        Effect::None | Effect::Quit | Effect::Batch(_) => None,
    }
}

// Convert EffectResult back to Msg
fn result_to_message(result: EffectResult) -> Msg {
    match result {
        EffectResult::CheckSerialization { artifact_index, result, output } => {
            Msg::CheckSerializationResult { artifact_index, result, output }
        }
        EffectResult::GeneratorFinished { artifact_index, result } => {
            Msg::GeneratorFinished { artifact_index, result }
        }
        // ... other variants
    }
}
```

### Pattern 3: Sequential FIFO in Async Task

**What:** Single background task processes effects in order using
`while let Some(cmd) = rx.recv().await`.

**When to use:** When effects have dependencies and must execute sequentially.

**Example:**

```rust
// Background task ensures FIFO ordering automatically
tokio::spawn(async move {
    let mut handler = BackgroundEffectHandler::new(backend, make);
    
    while let Some(cmd) = rx_cmd.recv().await {
        // Each command executes to completion before next is received
        let result = handler.execute(cmd).await;
        
        // Send result back (non-blocking)
        if tx_res.send(result).is_err() {
            break;
        }
    }
});
```

### Anti-Patterns to Avoid

- **Anti-pattern: `await` in TUI draw closure**: Terminal::draw() is
  synchronous; never .await inside it.
  - **Instead**: All async work happens in background task; TUI only handles
    messages.

- **Anti-pattern: Shared mutable state**: Don't use Arc<Mutex<Model>> - violates
  Elm architecture.
  - **Instead**: State stays in TUI; immutable data sent to background.

- **Anti-pattern: Blocking channel recv**: Don't use blocking_recv in async
  context.
  - **Instead**: Always use .await with tokio channels.

- **Anti-pattern: Multiple concurrent effects**: Don't spawn tasks per-effect;
  breaks FIFO.
  - **Instead**: Single background task processes sequentially.

## Don't Hand-Roll

| Problem               | Don't Build               | Use Instead             | Why                                               |
| --------------------- | ------------------------- | ----------------------- | ------------------------------------------------- |
| Channel communication | Custom sync primitives    | `tokio::sync::mpsc`     | Battle-tested, handles edge cases, optimized      |
| Async runtime         | Thread pool with channels | `tokio::runtime`        | Cooperative scheduling, integrated with ecosystem |
| Error propagation     | Custom Result types       | `anyhow::Result`        | Already used in codebase, ergonomic               |
| Event polling loop    | Busy waiting with sleep   | `tokio::select!`        | Proper async waiting, efficient                   |
| Effect serialization  | Manual byte encoding      | Rust enums over channel | Type-safe, zero-cost                              |

## Common Pitfalls

### Pitfall 1: Terminal Not Async-Safe

**What goes wrong:** `ratatui::Terminal` is not `Send + Sync` - cannot be
accessed from multiple threads/tasks.

**Why it happens:** Terminal uses internal mutable state and raw file
descriptors.

**How to avoid:** Terminal stays in main thread only; background task receives
immutable data, returns results.

**Warning signs:** Compiler error about `Send` trait not implemented.

### Pitfall 2: Channel Closed While Effects Pending

**What goes wrong:** TUI quits or crashes, background task panics when sending
to closed channel.

**Why it happens:** `send()` returns Err when receiver dropped; task must handle
gracefully.

**How to avoid:** Check send result, break loop on error:

```rust
if tx_res.send(result).is_err() {
    break; // TUI gone, exit cleanly
}
```

### Pitfall 3: Runtime Not Initialized

**What goes wrong:** `tokio::spawn` called without tokio runtime (#[tokio::main]
macro).

**Why it happens:** The binary entry point lacks tokio runtime setup.

**How to avoid:** Add `#[tokio::main]` to main() in src/bin/artifacts.rs:

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // ... existing code
}
```

### Pitfall 4: Blocking Operations in Async Context

**What goes wrong:** `std::fs` or `std::process` calls block the async task,
stalling all effects.

**Why it happens:** std operations are blocking; async task expects .await
points.

**How to avoid:** Use `tokio::task::spawn_blocking` for blocking I/O:

```rust
let output = tokio::task::spawn_blocking(|| {
    run_generator_script(...)
}).await??;
```

### Pitfall 5: Select! Biased Polling

**What goes wrong:** One branch of `tokio::select!` always ready, starving the
other.

**Why it happens:** `select!` polls futures in order; if events always ready,
channel starves.

**How to avoid:** Use `biased` keyword or fair polling; test with slow effects:

```rust
tokio::select! {
    biased; // Optional: explicit order
    msg = events.next_event_async() => { ... }
    result = res_rx.recv() => { ... }
}
```

## Code Examples

### Converting the EventSource trait for Async

```rust
// Source: Modified from existing src/tui/events.rs

#[async_trait::async_trait]
pub trait AsyncEventSource {
    async fn next_event(&mut self) -> Option<Msg>;
}

pub struct AsyncTerminalEventSource {
    tick_rate: Duration,
}

#[async_trait::async_trait]
impl AsyncEventSource for AsyncTerminalEventSource {
    async fn next_event(&mut self) -> Option<Msg> {
        // Use tokio's timeout for async polling
        match tokio::time::timeout(self.tick_rate, self.poll_event()).await {
            Ok(event) => event,
            Err(_) => Some(Msg::Tick), // Timeout = tick
        }
    }
}
```

### Background Task with Blocking I/O

```rust
// Source: Based on tokio docs pattern
use tokio::task::spawn_blocking;

async fn execute_check_serialization(
    entry: &ArtifactEntry,
    target: &str,
    backend: &BackendConfiguration,
    make: &MakeConfiguration,
) -> Result<EffectResult> {
    // Clone data for spawn_blocking (must be 'static)
    let entry = entry.clone();
    let target = target.to_string();
    let backend = backend.clone();
    let make = make.clone();
    
    // Run blocking operation in blocking thread pool
    let result = spawn_blocking(move || {
        run_check_serialization(&entry, &target, &backend, &make, "nixos")
    }).await?;
    
    Ok(EffectResult::CheckSerialization { ... })
}
```

### Runtime Loop Integration

```rust
// Source: Modified from existing src/tui/runtime.rs

pub async fn run_tui_async<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    mut model: Model,
) -> Result<RunResult> {
    // Initialize channels
    let (tx_cmd, rx_res) = spawn_background_task(backend, make);
    
    let mut event_source = AsyncTerminalEventSource::default();
    let mut frames_rendered = 0;
    
    loop {
        terminal.draw(|f| render(f, &model))?;
        frames_rendered += 1;
        
        tokio::select! {
            Some(msg) = event_source.next_event() => {
                let (new_model, effect) = update(model, msg);
                model = new_model;
                
                // Convert and send effect to background
                if let Some(cmd) = effect_to_command(effect) {
                    tx_cmd.send(cmd)?;
                }
            }
            Some(result) = rx_res.recv() => {
                let msg = result_to_message(result);
                let (new_model, _) = update(model, msg);
                model = new_model;
            }
            else => break, // Both channels closed
        }
        
        if model.should_quit {
            break;
        }
    }
    
    Ok(RunResult { final_model: model, frames_rendered })
}
```

## State of the Art

| Old Approach                      | Current Approach              | When Changed | Impact                                        |
| --------------------------------- | ----------------------------- | ------------ | --------------------------------------------- |
| Synchronous `EffectHandler` trait | Channel-based async           | Now          | TUI no longer blocks during long operations   |
| Blocking effect execution         | `tokio::task::spawn_blocking` | Now          | Proper async I/O without blocking the runtime |
| Single-threaded event loop        | `tokio::select!` multi-source | Now          | Concurrent event and result handling          |
| Direct backend calls              | Message-passing architecture  | Now          | Better separation, easier testing             |

**Deprecated/outdated:**

- `effect_handler.rs` synchronous implementation: Replaced by channel-based
  architecture
- `execute_effect()` blocking calls: Replaced by async message passing
- `NoOpEffectHandler` for testing: May still be useful, but tests can now mock
  channels

## Open Questions

1. **EventSource Async Conversion**
   - What we know: crossterm has async support via `EventStream`
   - What's unclear: Whether to use `crossterm::event::EventStream` or polling
     with timeout
   - Recommendation: Start with `EventStream` as it's the idiomatic async
     approach

2. **Error Handling in Channels**
   - What we know: Errors must be encapsulated in result messages
   - What's unclear: Whether to use `anyhow::Error` or custom error enum for
     channel errors
   - Recommendation: Use custom `EffectResult` enum with error variants per
     operation

3. **Graceful Shutdown**
   - What we know: Background task should exit when TUI closes
   - What's unclear: How to handle in-flight effects during shutdown
   - Recommendation: Drop command sender to signal shutdown; drain result
     receiver

## Sources

### Primary (HIGH confidence)

- `docs.rs/tokio/1.42.0/tokio/sync/mpsc/` - mpsc channel documentation,
  unbounded_channel API
- `docs.rs/ratatui/0.29.0/ratatui/` - Ratatui is not Send+Sync, Terminal is
  single-threaded
- `tokio.rs/tokio/tutorial/channels` - Official tokio channel tutorial with
  message-passing patterns
- `src/tui/runtime.rs` - Existing runtime.rs shows synchronous effect execution
  pattern
- `src/tui/effect_handler.rs` - Current blocking effect handler implementation
- `src/app/message.rs` - Existing Msg variants that map to EffectResult variants

### Secondary (MEDIUM confidence)

- `docs.rs/crossterm/0.28.0/crossterm/event/struct.EventStream.html` - Async
  event stream support
- Tokio tutorial code examples - Background task spawn patterns

### Tertiary (LOW confidence)

- None - all critical patterns verified from official sources

## Metadata

**Confidence breakdown:**

- Standard stack: HIGH - tokio mpsc is standard, ratatui architecture is
  documented
- Architecture: HIGH - Based on existing codebase patterns plus tokio best
  practices
- Pitfalls: MEDIUM-HIGH - Some edge cases (like graceful shutdown) will need
  testing

**Research date:** 2025-02-13 **Valid until:** 2025-03-13 (30 days - tokio and
ratatui are stable)

**Files analyzed:**

- `src/tui/runtime.rs` - Current TUI runtime loop
- `src/tui/effect_handler.rs` - Effect handler to be replaced
- `src/app/message.rs` - Message/Result types
- `src/app/effect.rs` - Effect variants
- `src/tui/events.rs` - Event handling
- `Cargo.toml` - Dependencies confirmed
