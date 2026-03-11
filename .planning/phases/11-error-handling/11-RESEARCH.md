# Phase 11: Error Handling Improvements - Research

**Researched:** 2026-02-18 **Domain:** Rust TUI Error Handling (ratatui +
crossterm) **Confidence:** HIGH

## Summary

Phase 11 focuses on ensuring TUI errors display properly to stderr without
polluting normal output, with comprehensive panic handling and terminal
restoration. This research covers five key requirements: initialization error
handling, terminal restoration on failure, runtime error display within the TUI,
panic handlers that restore terminal state, and log file output isolation.

The codebase already has foundational pieces in place: a `TerminalGuard` struct
with RAII-based restoration, an `install_panic_hook()` function, and a logging
infrastructure with feature-gated macros (`error!`, `warn!`, `info!`, `debug!`).
The Elm Architecture pattern used in the TUI separates concerns well, with
errors displayed in the Model's `error` field.

**Primary recommendation:** Implement a two-phase error handling strategy:
pre-terminal errors go directly to stderr, while post-terminal errors are
displayed in-TUI with a robust panic handler that always restores terminal state
before aborting.

---

## Current State Analysis

### Existing Error Handling Infrastructure

1. **TerminalGuard** (`src/tui/terminal.rs:15-60`)
   - RAII guard that enables raw mode and alternate screen on creation
   - Implements `Drop` trait for automatic restoration
   - Has `restore()` method for explicit cleanup
   - Already has `install_panic_hook()` and `restore_terminal()` functions

2. **Panic Hook** (`src/tui/terminal.rs:70-76`)
   - Currently installed in `run_tui()` at line 103
   - Calls `restore_terminal()` before invoking original hook
   - **Gap:** Does not print to stderr explicitly before calling original hook

3. **Logging System** (`src/logging.rs`)
   - Feature-gated macros: `error!`, `warn!`, `info!`, `debug!`
   - Writes to file when `--log-file` provided
   - **Gap:** No mechanism to suppress non-error output to stdout/stderr when
     logging

4. **CLI Entry Point** (`src/bin/artifacts.rs`)
   - Prints errors to stderr with `eprintln!` when logging feature disabled
   - **Gap:** Initialization errors (before TUI starts) may not be properly
     directed

5. **TUI Runtime** (`src/tui/runtime.rs`)
   - Uses `model.error` field for runtime errors
   - Errors displayed within TUI interface
   - **Gap:** No verification that errors never escape to stdout/stderr

---

## Requirements Breakdown

### ERR-01: TUI Initialization Failures

**Current:** Initialization happens in `run_tui()` after terminal setup\
**Gap:** Errors before terminal setup should print to stderr and exit non-zero

**Required Changes:**

1. Move configuration loading (backend.toml, flake.nix parsing) BEFORE terminal
   initialization
2. Print clear error messages to stderr using `eprintln!`
3. Exit with code 1 on failure

### ERR-02: Terminal Restoration Failures

**Current:** `TerminalGuard::restore()` returns `Result<()>`\
**Gap:** Errors in restoration are propagated but may not be printed to stderr

**Required Changes:**

1. In `restore()` error paths, print explicit error to stderr before returning
2. In `Drop` implementation, attempt restoration but ignore errors (can't
   propagate from Drop)
3. In panic handler, print restoration failure to stderr if it occurs

### ERR-03: Runtime Errors in TUI

**Current:** Errors stored in `model.error` and displayed in TUI\
**Gap:** No guarantee errors don't leak to stdout/stderr from background tasks

**Required Changes:**

1. Audit background task (`src/tui/background.rs`) for any stdout/stderr writes
2. Ensure all generator/serialization output is captured and stored, not printed
3. Verify `println!`/`eprintln!` are not used in TUI code paths

### ERR-04: Panic Handler Improvements

**Current:** Basic panic hook that restores terminal\
**Gap:** Does not print to stderr before calling original hook

**Required Changes:**

1. Print panic message to stderr explicitly in hook
2. Ensure terminal restoration happens even if original hook panics
3. Consider using `std::panic::resume_unwind` or proper abort after restoration

### UI-03: Log File Output Isolation

**Current:** Logging macros write to file but don't suppress other output\
**Gap:** Normal output (e.g., "No artifacts found") still goes to stdout

**Required Changes:**

1. Audit all `println!` calls in TUI code paths
2. When `--log-file` is provided:
   - Redirect normal output to log file only
   - Only errors go to stderr
3. Consider replacing `println!` with `info!` macro calls

---

## Architecture Patterns

### Pattern 1: Pre-Terminal Error Handling

**When to use:** Before `TerminalGuard::new()` is called\
**Pattern:**

```rust
// In src/cli/mod.rs

async fn run_tui(...) -> Result<()> {
    // Phase 1: Load configurations (NO TERMINAL YET)
    let backend = BackendConfiguration::read_backend_config(backend_path)
        .map_err(|e| {
            eprintln!("Failed to load backend configuration: {}", e);
            e
        })?;
    
    let make = MakeConfiguration::read_make_config(make_path)
        .map_err(|e| {
            eprintln!("Failed to load make configuration: {}", e);
            e
        })?;
    
    // Phase 2: Install panic hook BEFORE terminal setup
    install_panic_hook();
    
    // Phase 3: Initialize terminal
    let mut terminal_guard = TerminalGuard::new()
        .context("Failed to initialize terminal")?;
    
    // Phase 4: Run TUI
    // ...
}
```

### Pattern 2: Enhanced Panic Hook

**When to use:** Always installed before terminal setup\
**Pattern:**

```rust
// In src/tui/terminal.rs

pub fn install_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        // Always try to restore terminal first
        restore_terminal();
        
        // Print to stderr explicitly (UI-03 requirement)
        let payload = panic_info.payload();
        let message = if let Some(s) = payload.downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = payload.downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown panic".to_string()
        };
        
        eprintln!("Error: {}", message);
        
        // Call original hook
        original_hook(panic_info);
    }));
}
```

### Pattern 3: Terminal Restoration with Error Reporting

**When to use:** In `TerminalGuard::restore()` and `Drop`\
**Pattern:**

```rust
impl TerminalGuard {
    pub fn restore(&mut self) -> Result<()> {
        // Attempt each restoration step, reporting errors to stderr
        if let Err(e) = disable_raw_mode() {
            eprintln!("Warning: Failed to disable raw mode: {}", e);
        }
        
        if let Err(e) = execute!(self.terminal.backend_mut(), LeaveAlternateScreen) {
            eprintln!("Warning: Failed to leave alternate screen: {}", e);
        }
        
        if let Err(e) = self.terminal.show_cursor() {
            eprintln!("Warning: Failed to show cursor: {}", e);
        }
        
        Ok(())
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        // Best-effort restoration - can't report errors from Drop
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}
```

### Pattern 4: Output Suppression with --log-file

**When to use:** When `--log-file` CLI argument is provided\
**Pattern:**

```rust
// In src/cli/mod.rs::run_tui()

// Check if logging is enabled with --log-file
let log_file_enabled = cli.log_file.is_some();

// After TUI completes, handle output
match result {
    Ok(run_result) => {
        let failed: Vec<_> = // ... collect failed artifacts;
        
        if !failed.is_empty() {
            // Errors ALWAYS go to stderr
            eprintln!("Failed artifacts:");
            for msg in &failed {
                eprintln!("  {}", msg);
            }
        } else if !log_file_enabled {
            // Only print success if NO log file (UI-03)
            println!("All artifacts generated successfully");
        }
        // If log_file_enabled, success message goes to log file only
    }
    Err(e) => Err(e), // Errors propagate to main() which prints to stderr
}
```

---

## Don't Hand-Roll

| Problem              | Don't Build                        | Use Instead                         | Why                                                     |
| -------------------- | ---------------------------------- | ----------------------------------- | ------------------------------------------------------- |
| Panic handling       | Custom unwind catcher              | `std::panic::set_hook`              | Standard Rust mechanism, integrates with test framework |
| Terminal restoration | Manual cleanup in every error path | RAII `Drop` trait                   | Guarantees cleanup even with early returns              |
| Error formatting     | Custom error display               | `anyhow::Context` + `Display`       | Proper error chains, source location                    |
| Log level filtering  | Manual if-checks                   | `log` crate filters                 | Standard, configurable at runtime                       |
| Stdout suppression   | `std::io::set_output_capture`      | Conditional compilation or wrapping | `set_output_capture` is internal/unstable               |

---

## Common Pitfalls

### Pitfall 1: Terminal Restoration in Drop Swallows Errors

**What goes wrong:** Errors during `Drop::drop()` cannot be propagated, making
terminal restoration failures invisible\
**Why it happens:** `Drop::drop()` returns `()`, not `Result`\
**How to avoid:**

- Make `restore()` explicit and call it before letting guard drop
- In panic hook, call `restore_terminal()` explicitly before abort
- Accept that Drop-based restoration is best-effort only

### Pitfall 2: Panic Hook Double Panic

**What goes wrong:** If original panic hook panics, program aborts without
proper cleanup\
**Why it happens:** Rust aborts on double panic\
**How to avoid:**

- Use `std::panic::take_hook()` to get original hook
- Wrap original hook call in `catch_unwind` if possible (may not work in panic
  context)
- Ensure `restore_terminal()` is infallible

### Pitfall 3: Alternate Screen Hides Stderr

**What goes wrong:** When alternate screen is active, stderr output may not be
visible to user\
**Why it happens:** Alternate screen is a separate buffer; stderr goes to main
screen\
**How to avoid:**

- Never write to stderr while in alternate screen (except in panic handler after
  restoration)
- All runtime errors must go through TUI model.error field
- Only initialization errors and panics should use stderr

### Pitfall 4: Background Task Output Leaks

**What goes wrong:** Generator/serialization scripts may write to stdout/stderr
directly\
**Why it happens:** External scripts don't know about TUI\
**How to avoid:**

- Capture stdout/stderr when spawning child processes
- Use `std::process::Command::output()` instead of `status()` or `spawn()`
- Store captured output in model for display within TUI

### Pitfall 5: Initialization Order Dependencies

**What goes wrong:** Moving config loading before terminal setup may break error
reporting that relied on terminal\
**Why it happens:** Some error formatting may assume TUI context\
**How to avoid:**

- Keep error messages simple (strings only) during init phase
- Avoid complex error rendering before terminal is ready
- Use `anyhow` for error context, `eprintln!` for display

---

## Code Examples

### Example 1: Initialization Error Handling

```rust
// src/cli/mod.rs - Modified run_tui function

async fn run_tui(
    backend_path: &Path,
    make_path: &Path,
    machines: &[String],
    home_users: &[String],
    artifacts: &[String],
) -> Result<()> {
    // STEP 1: Load all configurations BEFORE terminal setup
    // Errors here print to stderr and exit non-zero
    let backend = BackendConfiguration::read_backend_config(backend_path)
        .with_context(|| {
            format!(
                "Failed to load backend configuration from '{}'",
                backend_path.display()
            )
        })?;
    
    let make = MakeConfiguration::read_make_config(make_path)
        .with_context(|| {
            format!(
                "Failed to load artifact definitions from nix evaluation",
            )
        })?;
    
    // STEP 2: Build model
    let mut model = build_filtered_model(&make, machines, home_users, artifacts);
    validate_model_capabilities(&mut model, &backend);
    
    if model.entries.is_empty() {
        // Use info! macro instead of println! for consistency
        #[cfg(feature = "logging")]
        crate::info!("No artifacts found matching filters");
        // Don't print to stdout - let caller handle empty result
        return Ok(());
    }
    
    // STEP 3: Install panic hook BEFORE terminal
    install_panic_hook();
    
    // STEP 4: Initialize terminal
    let mut terminal_guard = TerminalGuard::new()
        .context("Failed to initialize terminal for TUI")?;
    
    // STEP 5: Run TUI
    let result = run_async(/* ... */).await;
    
    // STEP 6: Explicit restoration with error reporting
    if let Err(e) = terminal_guard.restore() {
        eprintln!("Warning: Terminal restoration failed: {}", e);
    }
    
    // STEP 7: Handle result
    match result {
        Ok(run_result) => {
            // Report failures to stderr
            report_failed_artifacts(&run_result.final_model);
            Ok(())
        }
        Err(e) => {
            // Error will propagate to main() which prints to stderr
            Err(e)
        }
    }
}

fn report_failed_artifacts(model: &Model) {
    let failed: Vec<_> = model
        .artifacts
        .iter()
        .filter_map(|a| match &a.status {
            ArtifactStatus::Failed { error, .. } => {
                Some(format!("{}/{}: {}", a.target, a.artifact.name, error))
            }
            _ => None,
        })
        .collect();
    
    if !failed.is_empty() {
        eprintln!("Failed artifacts:");
        for msg in &failed {
            eprintln!("  {}", msg);
        }
    }
}
```

### Example 2: Enhanced Panic Hook

```rust
// src/tui/terminal.rs

/// Install a panic hook that:
/// 1. Restores terminal state
/// 2. Prints error to stderr
/// 3. Calls original hook
/// 4. Aborts cleanly
pub fn install_panic_hook() {
    let original_hook = std::panic::take_hook();
    
    std::panic::set_hook(Box::new(move |panic_info| {
        // CRITICAL: Restore terminal FIRST before any output
        restore_terminal();
        
        // Build panic message
        let payload = panic_info.payload();
        let message = if let Some(s) = payload.downcast_ref::<&str>() {
            (*s).to_string()
        } else if let Some(s) = payload.downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown panic occurred".to_string()
        };
        
        // Print location if available
        let location = panic_info.location()
            .map(|loc| format!(" at {}:{}", loc.file(), loc.line()))
            .unwrap_or_default();
        
        // Print to stderr (UI-03: errors to stderr)
        eprintln!("Panic: {}{}", message, location);
        
        // Call original hook (may print backtrace)
        original_hook(panic_info);
        
        // Ensure we exit (original hook may not)
        std::process::abort();
    }));
}
```

### Example 3: TerminalGuard with Explicit Error Reporting

```rust
// src/tui/terminal.rs

impl TerminalGuard {
    /// Restore terminal with explicit error reporting to stderr.
    /// This should be called explicitly before letting the guard drop.
    pub fn restore(&mut self) -> Result<()> {
        let mut had_error = false;
        
        // Disable raw mode
        if let Err(e) = disable_raw_mode() {
            eprintln!("Error: Failed to disable raw mode: {}", e);
            had_error = true;
        }
        
        // Leave alternate screen
        if let Err(e) = execute!(self.terminal.backend_mut(), LeaveAlternateScreen) {
            eprintln!("Error: Failed to leave alternate screen: {}", e);
            had_error = true;
        }
        
        // Show cursor
        if let Err(e) = self.terminal.show_cursor() {
            eprintln!("Error: Failed to show cursor: {}", e);
            had_error = true;
        }
        
        if had_error {
            Err(anyhow::anyhow!("Terminal restoration had errors"))
        } else {
            Ok(())
        }
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        // Best-effort cleanup - cannot report errors from Drop
        // These are infallible operations (return Result but we ignore)
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}
```

### Example 4: Output Suppression for --log-file

```rust
// src/cli/mod.rs

async fn run_tui(
    cli: &Cli,  // Pass CLI to check --log-file
    // ... other params
) -> Result<()> {
    let log_file_enabled = cli.log_file.is_some();
    
    // ... setup and run TUI ...
    
    match result {
        Ok(run_result) => {
            let failed = collect_failed_artifacts(&run_result);
            
            if !failed.is_empty() {
                // Errors ALWAYS go to stderr
                eprintln!("Failed artifacts:");
                for msg in &failed {
                    eprintln!("  {}", msg);
                }
                std::process::exit(1); // Non-zero exit on failure
            } else {
                // Success message only if no log file
                if !log_file_enabled {
                    println!("All artifacts generated successfully");
                } else {
                    // Goes to log file via info! macro
                    crate::info!("All artifacts generated successfully");
                }
            }
            Ok(())
        }
        Err(e) => Err(e),
    }
}
```

### Example 5: Runtime Error Display in TUI

```rust
// src/tui/views/mod.rs - Error display in render function

pub fn render(f: &mut Frame, model: &Model) {
    // ... other rendering ...
    
    // Display runtime errors in TUI (ERR-03)
    if let Some(error) = &model.error {
        let error_widget = Paragraph::new(error.as_str())
            .style(Style::default().fg(Color::Red))
            .block(Block::default().title("Error").borders(Borders::ALL));
        
        let area = centered_rect(60, 20, f.area());
        f.render_widget(Clear, area); // Clear background
        f.render_widget(error_widget, area);
    }
}
```

---

## File Structure Recommendations

### Changes Required by File

**`src/bin/artifacts.rs`**

- **Current:** Basic error printing to stderr
- **Changes:** None needed - already handles top-level errors correctly

**`src/cli/mod.rs`**

- **Current:** `run_tui()` loads configs, initializes terminal, runs TUI
- **Changes:**
  1. Add explicit error context to config loading
  2. Call `terminal_guard.restore()` explicitly before matching result
  3. Add `--log-file` check to suppress success messages
  4. Ensure `install_panic_hook()` called before `TerminalGuard::new()`

**`src/tui/terminal.rs`**

- **Current:** `TerminalGuard` with RAII, basic panic hook
- **Changes:**
  1. Enhance `install_panic_hook()` to print to stderr
  2. Update `restore()` to print errors to stderr
  3. Ensure `restore_terminal()` is infallible (used in panic hook)

**`src/tui/runtime.rs`**

- **Current:** Runtime loop with error handling
- **Changes:**
  1. Verify no `println!`/`eprintln!` in TUI code paths
  2. Ensure all errors go through `model.error`

**`src/tui/background.rs`** (if exists)

- **Audit:** Ensure child process output is captured, not leaked to
  stdout/stderr

---

## Testing Strategy

### Test Cases

1. **Initialization Failure Test**
   ```rust
   #[test]
   fn test_init_failure_prints_to_stderr() {
       // Create invalid backend.toml
       // Run CLI
       // Assert stderr contains error message
       // Assert exit code is non-zero
   }
   ```

2. **Panic Handler Test**
   ```rust
   #[test]
   fn test_panic_restores_terminal() {
       // Mock terminal
       // Install panic hook
       // Trigger panic
       // Assert terminal restored (raw mode disabled, alternate screen exited)
       // Assert stderr contains panic message
   }
   ```

3. **Log File Suppression Test**
   ```rust
   #[test]
   fn test_log_file_suppresses_stdout() {
       // Run with --log-file
       // Assert no stdout output
       // Assert log file contains normal messages
   }
   ```

4. **Terminal Restoration Failure Test**
   ```rust
   #[test]
   fn test_terminal_restore_failure_prints_error() {
       // Simulate terminal restore failure
       // Assert error printed to stderr
   }
   ```

---

## Open Questions

1. **Background Task Output Capture**
   - What: Current background task implementation
   - Unclear: Whether stdout/stderr from generator scripts is captured or leaked
   - Recommendation: Audit `src/tui/background.rs` and ensure output capture

2. **Error Display in TUI**
   - What: How runtime errors are displayed
   - Unclear: Whether all errors go through `model.error` or some use
     `eprintln!`
   - Recommendation: Search codebase for `eprintln!` and `println!` in
     `src/tui/`

3. **Exit Codes**
   - What: Exit code behavior on different failure modes
   - Unclear: Whether partial failures (some artifacts succeed) exit non-zero
   - Recommendation: Define clear exit code strategy (0 = all success, 1 = any
     failure)

---

## Sources

### Primary (HIGH confidence)

- **Codebase analysis** - Reviewed `src/tui/terminal.rs`, `src/cli/mod.rs`,
  `src/bin/artifacts.rs`, `src/logging.rs`, `src/tui/runtime.rs`
- **Ratatui documentation** - RAII patterns for terminal management
- **Crossterm documentation** - Raw mode and alternate screen handling

### Secondary (MEDIUM confidence)

- **Rust panic hook documentation** - `std::panic::set_hook` behavior
- **Drop trait semantics** - Error handling limitations in Drop

---

## Metadata

**Confidence breakdown:**

- Standard stack: HIGH - Based on existing codebase patterns
- Architecture: HIGH - Standard Rust error handling patterns
- Pitfalls: MEDIUM - Inferred from code structure, limited testing

**Research date:** 2026-02-18 **Valid until:** 2026-03-18 (30 days for stable
patterns)
