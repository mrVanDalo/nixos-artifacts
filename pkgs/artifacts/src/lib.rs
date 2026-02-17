pub mod app;
pub mod backend;
pub mod effect_handler;
pub mod logging;
#[macro_use]
pub mod macros;
pub mod cli;
pub mod config;
pub mod tui;

// Logging macros (error!, warn!, info!, debug!) are automatically exported
// at the crate root via #[macro_export] in src/logging.rs.
// They are feature-gated - when "logging" feature is disabled, they compile to nothing.

// Re-export Logger struct for programmatic access
pub use crate::logging::Logger;
