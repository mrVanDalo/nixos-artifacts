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
    /// for cleaner error handling.
    pub fn restore(&mut self) -> Result<()> {
        disable_raw_mode().context("Failed to disable raw mode")?;
        execute!(self.terminal.backend_mut(), LeaveAlternateScreen)
            .context("Failed to leave alternate screen")?;
        self.terminal
            .show_cursor()
            .context("Failed to show cursor")?;
        Ok(())
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
pub fn restore_terminal() {
    let _ = disable_raw_mode();
    let _ = execute!(io::stdout(), LeaveAlternateScreen);
}

/// Install a panic hook that restores the terminal before printing the panic.
pub fn install_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        restore_terminal();
        original_hook(panic_info);
    }));
}
