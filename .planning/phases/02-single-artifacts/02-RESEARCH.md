# Phase 2: Single Artifacts - Research

**Researched:** 2026-02-13 **Domain:** Async background task execution in
Rust/Tokio for TUI application **Confidence:** HIGH

## Summary

Phase 2 requires implementing three single-artifact effects (CheckSerialization,
RunGenerator, Serialize) with full script execution in the background. This
builds on Phase 1's channel infrastructure to enable non-blocking TUI operation.

The key challenge is integrating existing blocking backend operations (script
execution with bubblewrap) into the async background task architecture while
maintaining TUI responsiveness. The codebase already has well-defined structures
for this integration.

**Primary recommendation:** Use `tokio::task::spawn_blocking` to wrap the
synchronous script execution calls, convert existing backend operations to
return serializable results, and update the background handler to execute real
logic instead of stubs.

## User Constraints (from CONTEXT.md)

### Locked Decisions

- **Generation Initiation**: Press `Enter` on selected artifact to trigger
  generation
- **Confirmation prompt**: Appears only for regeneration (not first-time
  generation)
- **"a" key**: Triggers "generate all" with confirmation dialog
- **Generate all behavior**: Only generates artifacts that need generation,
  skips up-to-date ones
- **Regeneration prompt**: If artifact is up-to-date and user triggers
  generation → prompt "Regenerate and override old artifact? y/n"
- **Visual feedback**: Status symbol changes to "generating" state in list + log
  panel shows "Generating..." when artifact selected

- **Status Visibility**: Show current effect step: "CheckSerialization...",
  "Running generator...", "Serializing..."
- **Script output**: Shown only after completion, not streamed live
- **List updates**: Immediately with status symbol while generating
- **Full navigation**: User can scroll, select, and trigger other artifacts
  while one generates

- **Error Presentation**: Errors appear in log/detail panel AND in artifact's
  state symbol
- **Show full stdout + stderr**: For debugging purposes
- **Failed artifacts**: Show "Failed" status with retry option
- **Failed artifacts symbol**: Distinct color/symbol (e.g., red X or ⚠️ warning
  symbol)

- **Cancel/Abort Behavior**: Can quit TUI with confirmation: "Effects are
  running, quit anyway? y/n"
- **On quit**: Cancel immediately, except serialization effects which must
  complete
- **Cancel individual artifacts**: Can cancel with `c` or `Escape`, but running
  serialization effects continue
- **Duplicate generation requests**: Silently ignored (don't start duplicate)

### Claude's Discretion

- Exact symbols/colors for status states (generating, failed, success)
- Implementation of progress indicator animation (spinner style)
- Log panel scrolling behavior and capacity limits
- Confirmation dialog styling and key bindings

### Deferred Ideas

- None — discussion stayed within phase scope

## Standard Stack

### Core

| Library                     | Version | Purpose            | Why Standard                                         |
| --------------------------- | ------- | ------------------ | ---------------------------------------------------- |
| tokio                       | 1.x     | Async runtime      | Already in use, provides channels and spawn_blocking |
| tokio::sync::mpsc           | 1.x     | Unbounded channels | Phase 1 infrastructure, battle-tested                |
| tokio::task::spawn_blocking | 1.x     | CPU-bound work     | Wraps blocking script calls for async compatibility  |
| anyhow                      | 1.x     | Error handling     | Standard Rust error handling, used throughout        |
| ratatui                     | 0.29    | TUI framework      | Already integrated, handles responsive rendering     |

### Supporting

| Library               | Version | Purpose          | When to Use                           |
| --------------------- | ------- | ---------------- | ------------------------------------- |
| std::process::Command | std     | Script execution | Existing backend implementation       |
| std::sync::mpsc       | std     | Output capture   | Already in `output_capture.rs`        |
| tempfile              | 3.x     | Temp directories | Already in use for `TempFile` wrapper |

### Implementation Pattern

```rust
// Pattern: Wrap blocking code in spawn_blocking
async fn execute_async() -> Result<Output> {
    tokio::task::spawn_blocking(move || {
        // Blocking script execution here
        run_script_sync()
    }).await?
}
```

## Architecture Patterns

### Project Structure

```
pkgs/artifacts/src/
├── tui/
│   ├── background.rs      # BACKGROUND HANDLER (MODIFY THIS)
│   ├── channels.rs         # EffectCommand/EffectResult types
│   ├── runtime.rs          # Async runtime with tokio::select!
│   └── views/
│       └── list.rs         # Status symbols display
├── backend/
│   ├── generator.rs        # GENERATOR OPS (existing, blocking)
│   ├── serialization.rs    # SERIALIZATION OPS (existing, blocking)
│   ├── tempfile.rs         # TEMP FILE MANAGEMENT
│   └── output_capture.rs   # OUTPUT CAPTURE
├── app/
│   ├── model.rs            # Model/Status/LogStep types
│   ├── effect.rs           # Effect enum variants
│   ├── message.rs          # Msg enum for results
│   └── update.rs           # State transitions
```

### Pattern 1: Effect Execution Flow

**What:** Three-phase flow for artifact generation **When to use:** For every
single artifact generation

```rust
// Phase 1: CheckSerialization (background)
EffectCommand::CheckSerialization -> 
  EffectResult::CheckSerialization { needs_generation: bool }
    -> if needs_generation: trigger prompts/generator
    -> if up-to-date: mark as UpToDate

// Phase 2: RunGenerator (background, if needed)
EffectCommand::RunGenerator -> 
  EffectResult::GeneratorFinished { success: bool, output: String }
    -> if success: trigger Serialize
    -> if failed: mark as Failed

// Phase 3: Serialize (background, after generator)
EffectCommand::Serialize ->
  EffectResult::SerializeFinished { success: bool, error: Option<String> }
    -> if success: mark as UpToDate
    -> if failed: mark as Failed
```

### Pattern 2: Blocking Operation in Async Context

**What:** Use `spawn_blocking` to execute CPU-intensive or blocking operations
**When to use:** When calling existing blocking backend functions from async
code **Source:** `src/tui/background.rs` - `BackgroundEffectHandler::execute()`

```rust
// Current stub implementation (returns immediately):
pub async fn execute(&mut self, cmd: EffectCommand) -> EffectResult {
    match cmd {
        EffectCommand::CheckSerialization { artifact_index, ... } => {
            // TODO: Replace with actual implementation
            EffectResult::CheckSerialization {
                artifact_index,
                needs_generation: true,
                output: None,
            }
        }
        // ...
    }
}

// Required implementation pattern:
pub async fn execute(&mut self, cmd: EffectCommand) -> EffectResult {
    match cmd {
        EffectCommand::CheckSerialization { artifact_index, artifact_name, target, target_type } => {
            // Get artifact and config from self.make/self.backend
            let artifact = ...;
            let backend = self.backend.clone();
            let make = self.make.clone();
            
            // Spawn blocking task
            let result = tokio::task::spawn_blocking(move || {
                backend::serialization::run_check_serialization(
                    &artifact, &target, &backend, &make, &target_type
                )
            }).await;
            
            // Convert result to EffectResult
            match result {
                Ok(Ok(check_result)) => EffectResult::CheckSerialization {
                    artifact_index,
                    needs_generation: check_result.needs_generation,
                    output: Some(format_output(&check_result.output)),
                },
                Ok(Err(e)) => EffectResult::CheckSerialization {
                    artifact_index,
                    needs_generation: true, // Assume needs gen on error
                    output: Some(format!("Error: {}", e)),
                },
                Err(e) => // spawn_blocking panic
            }
        }
        // ...
    }
}
```

### Pattern 3: Effect-to-Command Conversion

**What:** Convert Effect enum to EffectCommand for channel transmission **When
to use:** In `tui/runtime.rs` before sending to background **Source:**
`src/tui/runtime.rs` - `effect_to_command()`

```rust
// Already implemented in Phase 1, verified working:
Effect::CheckSerialization { ... } -> 
  EffectCommand::CheckSerialization { artifact_index, artifact_name, target, target_type: target_type.to_string() }

Effect::RunGenerator { prompts, ... } ->
  EffectCommand::RunGenerator { prompts: prompts.clone(), ... }

Effect::Serialize { out_dir, ... } ->
  EffectCommand::Serialize { ... } // out_dir not needed in command (created in background)
```

### Pattern 4: Result-to-Message Conversion

**What:** Convert EffectResult to Msg for update loop **When to use:** In
`tui/runtime.rs` when receiving from background **Source:**
`src/tui/runtime.rs` - `result_to_message()`

```rust
// Already stubbed, needs actual output conversion:
EffectResult::CheckSerialization { artifact_index, needs_generation, output } =>
  Msg::CheckSerializationResult { artifact_index, result: Ok(needs_generation), output }

EffectResult::GeneratorFinished { artifact_index, success, output, error } =>
  Msg::GeneratorFinished { artifact_index, result: if success { Ok(output) } else { Err(error) } }

EffectResult::SerializeFinished { artifact_index, success, error } =>
  Msg::SerializeFinished { artifact_index, result: if success { Ok(()) } else { Err(error) } }
```

### Pattern 5: Status State Transitions

**What:** Model state transitions based on effect results **When to use:** In
`app/update.rs` when processing result messages **Source:** `src/app/model.rs` -
`ArtifactStatus` enum

```rust
// State machine:
Pending -> CheckSerializationResult(true) -> NeedsGeneration
Pending -> CheckSerializationResult(false) -> UpToDate
NeedsGeneration -> GeneratorFinished(Ok) -> Generating (awaiting Serialize)
NeedsGeneration -> GeneratorFinished(Err) -> Failed
Generating -> SerializeFinished(Ok) -> UpToDate
Generating -> SerializeFinished(Err) -> Failed
UpToDate -> (User triggers regen) -> Generating
Failed -> (User retries) -> Generating
```

### Anti-Patterns to Avoid

1. **Don't block the async runtime:** Never call blocking functions directly in
   async context. Always use `spawn_blocking`.

2. **Don't share mutable state between foreground/background:** The handler owns
   configuration (cloned), no shared mutable references.

3. **Don't panic in spawn_blocking:** Panics in blocking tasks will propagate as
   JoinError, handle gracefully.

4. **Don't stream output:** Per CONTEXT.md, output is buffered and returned
   complete, not streamed line-by-line during execution.

5. **Don't start duplicate generations:** Silently ignore requests for artifacts
   already in `Generating` state.

## Don't Hand-Roll

| Problem                   | Don't Build            | Use Instead                                | Why                                                |
| ------------------------- | ---------------------- | ------------------------------------------ | -------------------------------------------------- |
| Process output capture    | Custom async streams   | `output_capture::run_with_captured_output` | Already implemented, handles stdout/stderr merging |
| Temp directory management | Manual /tmp paths      | `backend::tempfile::TempFile`              | Secure, auto-cleanup, already integrated           |
| Script execution          | Direct Command::spawn  | `run_generator_script` / `run_serialize`   | Handles bubblewrap, env vars, error cases          |
| Bubblewrap isolation      | Manual namespace setup | Existing `bwrap` command in generator.rs   | Complex security requirements, already implemented |
| Check serialization       | Inline in handler      | `run_check_serialization`                  | Backend abstraction, supports all backends         |

**Key insight:** The backend operations are already fully implemented as
synchronous functions. The task is integrating them into the async background
infrastructure, not rewriting them.

## Common Pitfalls

### Pitfall 1: Blocking the Async Runtime

**What goes wrong:** Calling `run_generator_script` directly in async
`execute()` blocks all background processing **Why it happens:** Script
execution waits for subprocess completion, blocking the tokio runtime **How to
avoid:** Always wrap in `tokio::task::spawn_blocking()`

### Pitfall 2: Clone Costs for Large Config

**What goes wrong:** Cloning `BackendConfiguration` and `MakeConfiguration` for
every effect could be expensive **Why it happens:** These contain
HashMaps/BTreeMaps with all backend and artifact definitions **How to avoid:**
Config is already moved into BackgroundEffectHandler once at startup, not
per-effect. Just clone references as needed.

### Pitfall 3: Temp File Cleanup in Async Context

**What goes wrong:** `TempFile` implements Drop which may not run correctly
across await points **Why it happens:** TempFile cleanup happens on Drop, but
async moves may delay this **How to avoid:** Use explicit cleanup or ensure
TempFile drops at end of spawn_blocking scope

### Pitfall 4: Missing Output Conversion

**What goes wrong:** `CapturedOutput` from backend not converted to String for
EffectResult **Why it happens:** EffectResult expects `Option<String>` for
output, backend returns `CapturedOutput` **How to avoid:** Implement
`format_captured_output(&CapturedOutput) -> String` helper

### Pitfall 5: Generator Output Directory Lifetime

**What goes wrong:** Generator output dir deleted before Serialize can use it
**Why it happens:** TempFile drops at end of spawn_blocking, but Serialize
happens in separate spawn_blocking **How to avoid:** Create temp dir in
execute(), pass path to generator spawn_blocking, keep alive until serialize
completes or use persistent temp

### Pitfall 6: Artifact Lookup Failures

**What goes wrong:** artifact_index doesn't find artifact in model (race
condition or bug) **Why it happens:** Model may have changed between effect
start and result **How to avoid:** Include all needed data in EffectCommand,
don't rely on index lookups for critical data

### Pitfall 7: Serialization State Confusion

**What goes wrong:** Generator succeeds but serialize fails, artifact stuck in
failed state **Why it happens:** No retry path for serialize-only failures **How
to avoid:** Per CONTEXT.md, failed artifacts show retry option, user can
regenerate

## Code Examples

### Example 1: CheckSerialization Effect Implementation

```rust
// In src/tui/background.rs - BackgroundEffectHandler::execute()

EffectCommand::CheckSerialization {
    artifact_index,
    artifact_name,
    target,
    target_type,
} => {
    // Clone needed data for spawn_blocking
    let backend = self.backend.clone();
    let make = self.make.clone();
    
    // Spawn blocking task
    let result = tokio::task::spawn_blocking(move || {
        // Lookup artifact definition
        let artifact = make.get_artifact(&artifact_name, &target)?;
        
        // Run the check
        backend::serialization::run_check_serialization(
            &artifact,
            &target,
            &backend,
            &make,
            &target_type,
        )
    }).await;
    
    // Convert to EffectResult
    match result {
        Ok(Ok(check_result)) => EffectResult::CheckSerialization {
            artifact_index,
            needs_generation: check_result.needs_generation,
            output: Some(format_captured_output(&check_result.output)),
        },
        Ok(Err(e)) => EffectResult::CheckSerialization {
            artifact_index,
            needs_generation: true, // Conservative: assume needs gen on error
            output: Some(format!("Check failed: {}", e)),
        },
        Err(e) => EffectResult::CheckSerialization {
            artifact_index,
            needs_generation: true,
            output: Some(format!("Task panicked: {}", e)),
        },
    }
}
```

### Example 2: RunGenerator Effect Implementation

```rust
// In src/tui/background.rs - BackgroundEffectHandler::execute()

EffectCommand::RunGenerator {
    artifact_index,
    artifact_name,
    target,
    target_type,
    prompts,
} => {
    let backend = self.backend.clone();
    let make = self.make.clone();
    
    let result = tokio::task::spawn_blocking(move || -> Result<GeneratorResult> {
        use std::collections::HashMap;
        use tempfile::TempDir;
        
        // Get artifact
        let artifact = make.get_artifact(&artifact_name, &target)?;
        
        // Create temp directories
        let prompts_dir = TempDir::new()?;
        let out_dir = TempDir::new()?;
        
        // Write prompts to files
        for (name, value) in prompts {
            let path = prompts_dir.path().join(&name);
            std::fs::write(&path, value)?;
        }
        
        // Run generator
        let output = backend::generator::run_generator_script(
            &artifact,
            &target,
            &make.make_base,
            prompts_dir.path(),
            out_dir.path(),
            &target_type,
        )?;
        
        // Verify generated files
        backend::generator::verify_generated_files(&artifact, out_dir.path())?;
        
        // Return output and out_dir path (for serialize)
        Ok(GeneratorResult {
            output,
            out_dir: out_dir.into_path(), // Keep alive by returning
        })
    }).await;
    
    match result {
        Ok(Ok(gen_result)) => EffectResult::GeneratorFinished {
            artifact_index,
            success: true,
            output: Some(format_captured_output(&gen_result.output)),
            error: None,
        },
        Ok(Err(e)) => EffectResult::GeneratorFinished {
            artifact_index,
            success: false,
            output: None,
            error: Some(format!("Generator failed: {}", e)),
        },
        Err(e) => EffectResult::GeneratorFinished {
            artifact_index,
            success: false,
            output: None,
            error: Some(format!("Task panicked: {}", e)),
        },
    }
}
```

### Example 3: Helper for Converting CapturedOutput

```rust
// Helper function to convert backend output to display string
fn format_captured_output(output: &crate::backend::output_capture::CapturedOutput) -> String {
    use crate::backend::output_capture::OutputStream;
    
    let mut result = String::new();
    
    for line in &output.lines {
        let prefix = match line.stream {
            OutputStream::Stdout => "[stdout] ",
            OutputStream::Stderr => "[stderr] ",
        };
        result.push_str(prefix);
        result.push_str(&line.content);
        result.push('\n');
    }
    
    result.push_str(&format!("Exit: {}\n", if output.exit_success { "success" } else { "failure" }));
    
    result
}
```

### Example 4: Status Display (from existing code)

```rust
// From src/tui/views/list.rs - status_display()

fn status_display(status: &ArtifactStatus) -> (&'static str, Style) {
    match status {
        ArtifactStatus::Pending => ("○", Style::default().fg(Color::Gray)),
        ArtifactStatus::NeedsGeneration => ("◐", Style::default().fg(Color::Yellow)),
        ArtifactStatus::UpToDate => ("✓", Style::default().fg(Color::Green)),
        ArtifactStatus::Generating => ("⟳", Style::default().fg(Color::Cyan)),
        ArtifactStatus::Failed(_) => ("✗", Style::default().fg(Color::Red)),
    }
}
```

## State of the Art

### Current Implementation Status

| Component                     | Status      | Notes                                               |
| ----------------------------- | ----------- | --------------------------------------------------- |
| Channel infrastructure        | ✅ Complete | Phase 1 implemented unbounded mpsc channels         |
| Background task spawning      | ✅ Complete | `spawn_background_task()` in `background.rs`        |
| Effect-to-Command conversion  | ✅ Complete | `effect_to_command()` in `runtime.rs`               |
| Result-to-Message conversion  | ✅ Stubbed  | `result_to_message()` exists but needs work         |
| Backend operations (blocking) | ✅ Complete | `generator.rs`, `serialization.rs` fully functional |
| Output capture                | ✅ Complete | `output_capture.rs` handles stdout/stderr merging   |
| Model state transitions       | ✅ Complete | `update.rs` handles all message types               |
| Status display                | ✅ Complete | `list.rs` shows symbols, log panel shows details    |

### What Phase 2 Actually Needs to Do

1. **Replace stubs in `background.rs`**: Change `execute()` from returning
   hardcoded values to calling real backend operations wrapped in
   `spawn_blocking`

2. **Wire up temp directory management**: Ensure generator output survives from
   `RunGenerator` to `Serialize` effect

3. **Convert output formats**: `CapturedOutput` -> `String` for channel
   transmission

4. **Handle error cases**: Convert backend errors to EffectResult error variants

5. **Test the flow**: Verify Check -> (if needed) Generator -> Serialize
   pipeline works end-to-end

## Sources

### Primary (HIGH confidence)

- `pkgs/artifacts/src/tui/background.rs` - Background task handler with stubs
- `pkgs/artifacts/src/tui/channels.rs` - EffectCommand/EffectResult definitions
- `pkgs/artifacts/src/tui/runtime.rs` - Async runtime with conversion functions
- `pkgs/artifacts/src/backend/generator.rs` - Blocking generator operations
- `pkgs/artifacts/src/backend/serialization.rs` - Blocking serialization
  operations
- `pkgs/artifacts/src/backend/output_capture.rs` - Output capture mechanism
- `pkgs/artifacts/src/app/model.rs` - Model and status definitions
- `pkgs/artifacts/Cargo.toml` - Dependency versions

### Secondary (MEDIUM confidence)

- `CLAUDE.md` in `pkgs/artifacts/` - Architecture documentation
- Phase 1 CONTEXT.md - Design decisions for channel infrastructure

### Tertiary (LOW confidence)

- None needed - all critical code is in the codebase

## Metadata

**Confidence breakdown:**

- Standard stack: HIGH - tokio already in use, versions verified
- Architecture: HIGH - patterns already established in Phase 1
- Pitfalls: HIGH - clear from reviewing existing async/sync boundary

**Research date:** 2026-02-13 **Valid until:** 30 days (stable stack)

## Open Questions

1. **Temp directory lifetime across effects**
   - What we know: Generator needs out_dir, Serialize needs to read from it
   - What's unclear: Best way to pass path from RunGenerator result to Serialize
     command
   - Recommendation: Store out_dir path in Model's GeneratingState, pass to
     Serialize effect

2. **Aggregate CheckResult output format**
   - What we know: CheckResult has `output: CapturedOutput` field
   - What's unclear: Whether to show full output or just summary in UI
   - Recommendation: Convert to string, show in log panel under "Check" step

3. **Error recovery for partial failures**
   - What we know: Generator can fail, Serialize can fail
   - What's unclear: Whether to allow retry from specific step or full
     regeneration
   - Recommendation: Per CONTEXT.md, failed artifacts show "Failed" with retry
     option - always restart from Check

4. **Generator verification order**
   - What we know: `verify_generated_files` checks expected vs actual files
   - What's unclear: Whether this happens in background before or after
     generator returns
   - Recommendation: Run verification in same spawn_blocking as generator,
     return error if verification fails
