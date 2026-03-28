//! Terminal UI module for the artifacts application.
//!
//! This module provides the terminal user interface implementation
//! using the ratatui framework. It includes event handling, runtime,
//! terminal management, and effect handling.
//!
//! ## Submodules
//!
//! - `events` - Event source trait and implementations
//! - `runtime` - Main runtime loop for the TUI
//! - `terminal` - Terminal setup and teardown
//! - `background` - Executes side effects from the app
//! - `channels` - Async communication for effects
//! - `model_builder` - Builds the app model from configuration
//! - `views` - UI view components (list, prompt, progress, etc.)
//! - `effect_handler` - Backend effect handler for executing real operations

pub mod background;
pub mod channels;
pub mod effect_handler;
pub mod events;
pub mod model_builder;
pub mod runtime;
pub mod terminal;
pub mod views;

pub use effect_handler::BackendEffectHandler;
pub use events::{EventSource, ScriptedEventSource, TerminalEventSource};
pub use model_builder::{build_model, build_model_with_validation, validate_model_capabilities};
pub use runtime::{EffectHandler, NoOpEffectHandler, RunResult, run, run_async, simulate};
pub use terminal::{AppTerminal, TerminalGuard, install_panic_hook, restore_terminal};
pub use views::render;
