//! Application state management using the Elm Architecture pattern.
//!
//! This module implements the Elm Architecture (Model-Update-View) for the
//! artifacts CLI application. All state transitions are pure functions,
//! making the application predictable and testable.
//!
//! # Architecture Overview
//!
//! The Elm Architecture separates concerns into three core concepts:
//!
//! 1. **Model** ([`model::Model`]): The single source of truth for application state
//! 2. **Update** ([`update::update`]): Pure functions that transform `(Model, Message) -> (Model, Effect)`
//! 3. **View** (in [`crate::tui::views`]): Renders the current model to the terminal
//!
//! # Key Design Principles
//!
//! - **Immutable State**: Models are never mutated in place; new states are computed
//! - **Pure Updates**: The `update` function has no side effects - effects are returned as data
//! - **Message-Driven**: All state changes flow through the [`message::Message`] enum
//! - **Effects as Data**: Side effects ([`effect::Effect`]) are descriptors executed by the runtime
//!
//! # Module Structure
//!
//! - [`model`]: State types including [`model::Screen`], [`model::ArtifactStatus`],
//!   [`model::ListEntry`], and the root [`model::Model`]
//! - [`message`]: Event types ([`message::Message`], [`message::KeyEvent`]) and captured output
//! - [`effect`]: Side effect descriptors that the runtime executes
//! - `update`: The pure update function and screen-specific handlers
//!
//! # Example Flow
//!
//! ```text
//! User presses 'j' → KeyEvent → Message::Key → update() → new Model + Effect::None
//!                    ↓
//!              TUI renders new Model
//! ```
//!
//! For the runtime implementation that executes effects, see [`crate::tui::runtime`].

pub mod effect;
pub mod message;
pub mod model;
pub mod update;

// Re-exports for convenient access from other modules
pub use effect::Effect;
pub use message::{KeyEvent, Message};
pub use model::{ArtifactEntry, ArtifactStatus, InputMode, Model, PromptState, Screen};
pub use update::{init, update};
