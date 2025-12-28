pub mod effect_handler;
pub mod events;
pub mod model_builder;
pub mod runtime;
pub mod terminal;
pub mod views;

pub use effect_handler::BackendEffectHandler;
pub use events::{EventSource, ScriptedEventSource, TerminalEventSource};
pub use model_builder::{build_filtered_model, build_model};
pub use runtime::{run, simulate, EffectHandler, NoOpEffectHandler, RunResult};
pub use terminal::{install_panic_hook, restore_terminal, AppTerminal, TerminalGuard};
pub use views::render;
