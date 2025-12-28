pub mod effect_handler;
pub mod events;
pub mod model_builder;
pub mod runtime;
pub mod terminal;
pub mod views;

pub use effect_handler::BackendEffectHandler;
pub use events::{EventSource, ScriptedEventSource, TerminalEventSource};
pub use model_builder::{build_filtered_model, build_model};
pub use runtime::{EffectHandler, NoOpEffectHandler, RunResult, run, simulate};
pub use terminal::{AppTerminal, TerminalGuard, install_panic_hook, restore_terminal};
pub use views::render;
