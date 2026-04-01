//! Application state types for the Elm Architecture implementation.
//!
//! This module defines all the state types used by the application:
//! - [`Model`]: The root application state
//! - [`Screen`]: Current view/screen being displayed
//! - [`ArtifactStatus`]: Lifecycle state of artifact generation
//! - [`ListEntry`]: Unified list entry type (single or shared artifacts)
//! - Various screen states for prompts, generation, logs, etc.
//!
//! # State Immutability
//!
//! All types in this module are designed for immutability:
//! - All fields are `pub` for pattern matching
//! - Update functions create new instances rather than mutate
//! - Cloning is cheap (most fields are small or reference-counted)

mod artifact;
mod core;
mod log;
mod prompt;
mod screen_state;
mod target;

pub use artifact::*;
pub use core::*;
pub use log::*;
pub use prompt::*;
pub use screen_state::*;
pub use target::*;
