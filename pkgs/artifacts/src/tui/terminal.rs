use anyhow::{Context, Result};
use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io::{self, Stdout};

/// Type alias for our terminal type
pub type AppTerminal = Terminal<CrosstermBackend<Stdout>>;

/// RAII guard that manages terminal state.
/// Enables raw mode and alternate screen on creation,
/// restores terminal on drop.
pub struct TerminalGuard {
    terminal: AppTerminal,
}

impl TerminalGuard {
    /// Initialize the terminal for TUI mode.
    /// This enables raw mode and switches to the alternate screen.
    pub fn new() -> Result<Self> {
        enable_raw_mode().context("Failed to enable raw mode")?;

        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen).context("Failed to enter alternate screen")?;

        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend).context("Failed to create terminal")?;

        Ok(Self { terminal })
    }

    /// Get a mutable reference to the terminal for rendering
    pub fn terminal(&mut self) -> &mut AppTerminal {
        &mut self.terminal
    }

    /// Restore the terminal to its original state.
    /// This is called automatically on drop, but can be called explicitly
    /// for cleaner error handling. Prints errors to stderr (ERR-02).
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
        // Best-effort restoration on drop
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}

/// Restore terminal state after a panic or error.
/// Call this in a panic hook to ensure the terminal is usable after a crash.
/// This function is infallible - it ignores all errors to ensure cleanup
/// happens even in panic contexts.
pub fn restore_terminal() {
    let _ = disable_raw_mode();
    let _ = execute!(io::stdout(), LeaveAlternateScreen);
}

/// Install a panic hook that:
/// 1. Restores terminal state
/// 2. Prints error to stderr
/// 3. Calls original hook
pub fn install_panic_hook() {
    let original_hook = std::panic::take_hook();

    std::panic::set_hook(Box::new(move |panic_info| {
        // CRITICAL: Restore terminal FIRST before any output (ERR-04)
        restore_terminal();

        // Build panic message from payload
        let payload = panic_info.payload();
        let message = if let Some(s) = payload.downcast_ref::<&str>() {
            (*s).to_string()
        } else if let Some(s) = payload.downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown panic occurred".to_string()
        };

        // Print location if available
        let location = panic_info
            .location()
            .map(|loc| format!(" at {}:{}", loc.file(), loc.line()))
            .unwrap_or_default();

        // Print to stderr (ERR-04: errors to stderr)
        eprintln!("Error: Panic occurred: {}{}", message, location);

        // Call original hook (may print backtrace)
        original_hook(panic_info);
    }));
}
