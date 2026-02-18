# Phase 12: Script Output Visibility - Research

**Researched:** 2026-02-18  
**Domain:** Rust TUI Application with ratatui, Async Script Execution  
**Confidence:** HIGH

## Summary

The artifacts CLI already has substantial infrastructure for script output capture and display. Research reveals that:

1. **Output Capture Infrastructure Exists**: `backend/output_capture.rs` provides `CapturedOutput` and `OutputLine` types that already capture stdout/stderr from scripts with stream identification.

2. **Channel-Based Async Architecture**: The TUI uses tokio channels (`EffectCommand`/`EffectResult`) to communicate between foreground (UI) and background (script execution) tasks, enabling non-blocking script execution.

3. **Partial Output Integration**: Output is already being captured in `effect_handler.rs` and returned via messages (`GeneratorOutput`, `SerializeOutput`, `CheckOutput`), but conversion in `runtime.rs` discards much of this data.

4. **Log Display Infrastructure**: The list view (`list.rs`) already has a log panel that displays `StepLogs` with accordion-style sections for Check/Generate/Serialize steps.

5. **Model Storage for Output**: `StepLogs` struct in `model.rs` provides storage organized by step (check, generate, serialize) with `LogEntry` supporting different log levels (Info, Output, Error, Success).

**Primary recommendation:** Complete the data flow from background script execution through channel results to model storage, then enhance the log panel display to show real-time updates during script execution and historical output in artifact detail view.

---

## Current Architecture Analysis

### Script Output Capture (COMPLETE)

**Location**: `pkgs/artifacts/src/backend/output_capture.rs`

The `run_with_captured_output()` function already captures both stdout and stderr:

```rust
pub struct CapturedOutput {
    pub lines: Vec<OutputLine>,
    pub exit_success: bool,
}

pub struct OutputLine {
    pub stream: OutputStream,  // Stdout or Stderr
    pub content: String,
}
```

**Key capability**: Uses separate threads with `mpsc::channel` to merge stdout/stderr in approximate arrival order.

### Effect Handler Integration (PARTIAL)

**Location**: `pkgs/artifacts/src/tui/effect_handler.rs`

The effect handler already captures output and returns structured results:

```rust
fn run_generator_and_store_output(...) -> Result<GeneratorOutput, String> {
    let captured = run_generator_script(...)?;
    let (stdout_lines, stderr_lines) = split_captured_output(&captured);
    
    Ok(GeneratorOutput {
        stdout_lines,
        stderr_lines,
        files_generated,
    })
}
```

**Gap**: Output is captured but only split into stdout/stderr vectors - the interleaved stream order is lost.

### Channel Communication (NEEDS ENHANCEMENT)

**Location**: `pkgs/artifacts/src/tui/channels.rs`

Current `EffectResult` variants already include output fields:

```rust
pub enum EffectResult {
    GeneratorFinished {
        artifact_index: usize,
        success: bool,
        output: Option<String>,  // <-- Present but underutilized
        error: Option<String>,
    },
    // ... similar for other variants
}
```

**Location**: `pkgs/artifacts/src/tui/background.rs`

The background task executes effects and returns results, but output formatting is basic.

### Runtime Conversion (HAS GAPS)

**Location**: `pkgs/artifacts/src/tui/runtime.rs`

The `result_to_message()` function currently has TODOs for output conversion:

```rust
EffectResult::GeneratorFinished { output, ... } => {
    let result = if success {
        Ok(GeneratorOutput {
            stdout_lines: output
                .unwrap_or_default()
                .lines()
                .map(|s| s.to_string())
                .collect(),
            stderr_lines: vec![],  // <-- Lost!
            files_generated: 0,   // <-- Lost!
        })
    } else { ... }
}
```

**Gap**: Converting from `EffectResult` to `Msg` loses stderr separation and interleaving information.

### Model Storage (COMPLETE)

**Location**: `pkgs/artifacts/src/app/model.rs`

Robust log storage exists:

```rust
pub struct StepLogs {
    pub check: Vec<LogEntry>,
    pub generate: Vec<LogEntry>,
    pub serialize: Vec<LogEntry>,
}

pub struct LogEntry {
    pub level: LogLevel,    // Info, Output, Error, Success
    pub message: String,
}
```

### Display Views (COMPLETE)

**Location**: `pkgs/artifacts/src/tui/views/list.rs`

The log panel already supports:
- Accordion-style step display (Check/Generate/Serialize)
- Log level indicators (i/|/!/✓)
- Auto-scroll to latest logs
- Error detail display with output

---

## Architecture Patterns

### Pattern 1: Preserve Stream Interleaving

**Current**: Output is split into separate stdout/stderr vectors after capture.

**Better**: Preserve the `Vec<OutputLine>` with stream markers to show output in true execution order with stream identification.

```rust
pub struct ScriptOutput {
    pub lines: Vec<OutputLine>,  // Preserves interleaving
}

pub enum OutputLine {
    Stdout(String),
    Stderr(String),
}
```

### Pattern 2: Streaming Updates via Periodic Polling

For real-time display during script execution:

1. Execute script with `tokio::process::Command` (async)
2. Stream stdout/stderr via `tokio::io::AsyncBufReadExt::lines()`
3. Send incremental `Msg::ScriptOutputLine` messages via channel
4. Update model with new lines during execution
5. Final `Msg::ScriptFinished` completes the operation

**Note**: Current architecture waits for script completion before updating. Real-time streaming requires switching to async process execution.

### Pattern 3: Unified Output Storage

Store output in `StepLogs` as it's generated:

```rust
impl StepLogs {
    pub fn append_output(&mut self, step: LogStep, lines: Vec<OutputLine>) {
        for line in lines {
            let (level, message) = match line.stream {
                OutputStream::Stdout => (LogLevel::Output, line.content),
                OutputStream::Stderr => (LogLevel::Error, line.content),
            };
            self.get_mut(step).push(LogEntry { level, message });
        }
    }
}
```

---

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Async process execution | Manual thread spawning | `tokio::process::Command` | Built-in async support, cancellation, proper cleanup |
| Output line buffering | Manual byte buffering | `tokio::io::AsyncBufReadExt::lines()` | Proper UTF-8 handling, async iteration |
| Terminal scrolling | Custom scroll logic | `Paragraph::scroll()` in ratatui | Standard widget behavior, tested |
| Log storage | Custom data structures | Existing `StepLogs` + `LogEntry` | Already integrated with views |
| Channel communication | Raw channels | Existing `EffectCommand`/`EffectResult` | Already integrated with runtime |

---

## Common Pitfalls

### Pitfall 1: Blocking the UI Thread

**What goes wrong**: Running scripts synchronously in the effect handler freezes the TUI, preventing real-time updates.

**Why it happens**: Current `effect_handler.rs` uses synchronous `run_generator_script()` which blocks until completion.

**How to avoid**: Use `tokio::process::Command` in the background task (`background.rs`) and send incremental output messages.

**Warning signs**: UI unresponsive during script execution, no spinner animation.

### Pitfall 2: Buffering All Output in Memory

**What goes wrong**: Scripts with massive output (e.g., verbose logging) consume excessive memory.

**Why it happens**: Storing every line indefinitely in `StepLogs`.

**How to avoid**: Implement ring buffer or truncation for completed artifacts (keep last N lines, summarize earlier output).

### Pitfall 3: Losing Stream Identity

**What goes wrong**: Merging stdout/stderr into single string loses which stream each line came from.

**Why it happens**: `EffectResult::GeneratorFinished { output: Option<String> }` is a single string.

**How to avoid**: Use structured output preserving stream identity through the entire pipeline:
```rust
pub struct ScriptOutput {
    pub lines: Vec<(OutputStream, String)>,
}
```

### Pitfall 4: Race Conditions in Log Display

**What goes wrong**: Scrolling log display shows inconsistent state if model updates mid-render.

**Why it happens**: Rendering happens in `terminal.draw()` while background tasks mutate model via messages.

**How to avoid**: Ensure all model updates happen in `update()` function (single-threaded), and use `tokio::sync::mpsc` for thread-safe message passing.

---

## Implementation Path

### Step 1: Complete Data Flow (MEDIUM)

Modify channel types to preserve full output:

```rust
// In channels.rs
pub struct ScriptOutput {
    pub lines: Vec<(OutputStream, String)>,
    pub truncated: bool,  // If output exceeded max size
}

pub enum EffectResult {
    GeneratorFinished {
        artifact_index: usize,
        success: bool,
        stdout: ScriptOutput,
        stderr: ScriptOutput,
        error: Option<String>,
    },
    // ... similar for other variants
}
```

### Step 2: Update Runtime Conversion (MEDIUM)

Complete `result_to_message()` in `runtime.rs` to pass output through:

```rust
EffectResult::GeneratorFinished { stdout, stderr, ... } => {
    let output = GeneratorOutput {
        stdout_lines: stdout.lines.into_iter().map(|(_, s)| s).collect(),
        stderr_lines: stderr.lines.into_iter().map(|(_, s)| s).collect(),
        interleaved_lines: interleave_streams(stdout.lines, stderr.lines),
        files_generated: /* get from artifact */,
    };
    Msg::GeneratorFinished { artifact_index, result: Ok(output) }
}
```

### Step 3: Store Output in Model (EASY)

Update `update.rs` to store output in `StepLogs`:

```rust
Msg::GeneratorFinished { artifact_index, result: Ok(output) } => {
    if let Some(entry) = model.entries.get_mut(artifact_index) {
        entry.step_logs_mut().append_output(
            LogStep::Generate,
            &output.interleaved_lines
        );
    }
    // ... rest of update logic
}
```

### Step 4: Real-Time Streaming (HARD)

For real-time updates, refactor script execution to be async:

```rust
// In background.rs
async fn run_generator_streaming(
    artifact_index: usize,
    // ... other params
    tx: mpsc::UnboundedSender<EffectResult>,
) {
    let mut child = tokio::process::Command::new(...)
        .spawn()
        .unwrap();
    
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    
    // Stream lines as they arrive
    let mut stdout_lines = BufReader::new(stdout).lines();
    let mut stderr_lines = BufReader::new(stderr).lines();
    
    loop {
        tokio::select! {
            line = stdout_lines.next_line() => {
                if let Ok(Some(line)) = line {
                    tx.send(EffectResult::OutputLine {
                        artifact_index,
                        stream: OutputStream::Stdout,
                        content: line,
                    }).ok();
                }
            }
            line = stderr_lines.next_line() => {
                if let Ok(Some(line)) = line {
                    tx.send(EffectResult::OutputLine {
                        artifact_index,
                        stream: OutputStream::Stderr,
                        content: line,
                    }).ok();
                }
            }
        }
    }
}
```

**Note**: This requires adding new `EffectResult::OutputLine` variant and handling it in `result_to_message()`.

---

## Code Examples

### Current Pattern (Captures but doesn't display fully)

```rust
// effect_handler.rs - capture is good
let captured = run_generator_script(...)?;
let (stdout_lines, stderr_lines) = split_captured_output(&captured);

// runtime.rs - conversion loses data
EffectResult::GeneratorFinished { output, ... } => {
    Ok(GeneratorOutput {
        stdout_lines: output.unwrap_or_default().lines()...,
        stderr_lines: vec![],  // Lost!
        ...
    })
}
```

### Improved Pattern (Preserves all data)

```rust
// Define unified output structure
pub struct ScriptOutput {
    pub lines: Vec<OutputLine>,  // Preserves interleaving
}

// Store in EffectResult
pub enum EffectResult {
    GeneratorFinished {
        artifact_index: usize,
        success: bool,
        output: ScriptOutput,  // Full structured output
        error: Option<String>,
    },
}

// Convert to message preserving structure
pub struct GeneratorOutput {
    pub stdout_lines: Vec<String>,
    pub stderr_lines: Vec<String>,
    pub interleaved: Vec<OutputLine>,  // For chronological display
    pub files_generated: usize,
}

// Store in StepLogs with proper level
impl StepLogs {
    pub fn append_script_output(&mut self, step: LogStep, output: &ScriptOutput) {
        for line in &output.lines {
            let log_entry = match line.stream {
                OutputStream::Stdout => LogEntry::output(&line.content),
                OutputStream::Stderr => LogEntry::error(&line.content),
            };
            self.get_mut(step).push(log_entry);
        }
    }
}
```

---

## Open Questions

1. **Real-time vs At-End Display**
   - What we know: Current system captures output at end
   - What's unclear: Whether real-time streaming is required for v3.0 or can be deferred
   - Recommendation: Implement at-end display first (easier), then streaming in later phase

2. **Output Retention Policy**
   - What we know: `StepLogs` stores all logs indefinitely
   - What's unclear: Whether to truncate/summarize old output for memory efficiency
   - Recommendation: Keep full output for now (debugging is primary use case)

3. **Shared Artifact Output Aggregation**
   - What we know: Shared artifacts run once and serialize to multiple targets
   - What's unclear: How to display output from multiple target checks in detail view
   - Recommendation: Store per-target check output in `SharedEntry`, aggregate in display

---

## Sources

### Primary (HIGH confidence)

- `pkgs/artifacts/src/backend/output_capture.rs` - Complete output capture implementation
- `pkgs/artifacts/src/tui/effect_handler.rs` - Effect execution with output capture
- `pkgs/artifacts/src/app/model.rs` - Model structures including StepLogs, LogEntry
- `pkgs/artifacts/src/tui/views/list.rs` - Log panel display implementation
- `pkgs/artifacts/src/tui/channels.rs` - Channel message types
- `pkgs/artifacts/src/tui/runtime.rs` - Message conversion and runtime loop
- `pkgs/artifacts/src/tui/background.rs` - Background task execution

### Secondary (MEDIUM confidence)

- Ratatui documentation (via training): `Paragraph::scroll()`, `List` widgets
- Tokio documentation (via training): `tokio::process::Command`, `AsyncBufReadExt::lines()`

---

## Metadata

**Confidence breakdown:**

- Standard stack: HIGH - Codebase already uses tokio, ratatui, standard patterns
- Architecture: HIGH - Existing patterns are clear and well-structured
- Pitfalls: MEDIUM - Some edge cases identified, may have missed others

**Research date:** 2026-02-18  
**Valid until:** 30 days (stable codebase)

---

## RESEARCH COMPLETE

**Phase:** 12 - Script Output Visibility  
**Confidence:** HIGH

### Key Findings

1. **Strong Foundation**: 80% of the infrastructure already exists - output capture, channel communication, model storage, and display views are all in place.

2. **Main Gap**: Data flow from `EffectResult` through `result_to_message()` to `StepLogs` needs completion. Currently output is captured but lost in conversion.

3. **Two-Tier Approach**: 
   - **Tier 1 (Immediate)**: Complete the data flow to show output after script completion (matches requirement OUT-04)
   - **Tier 2 (Advanced)**: Implement async streaming for real-time updates (requirement OUT-03)

4. **No New Dependencies**: Can be implemented with existing tokio and ratatui capabilities.

### File Created

`.planning/phases/12-script-output-visibility/12-RESEARCH.md`

### Confidence Assessment

| Area | Level | Reason |
|------|-------|--------|
| Standard stack | HIGH | Existing codebase patterns are clear and consistent |
| Architecture | HIGH | Complete understanding of data flow from capture to display |
| Pitfalls | MEDIUM | Identified common issues, some edge cases may exist |

### Open Questions

1. Real-time streaming vs at-end display priority
2. Output retention/truncation policy for memory management
3. Shared artifact multi-target output display strategy

### Ready for Planning

Research complete. Planner can now create PLAN.md files with specific tasks for completing the data flow and enhancing display.
