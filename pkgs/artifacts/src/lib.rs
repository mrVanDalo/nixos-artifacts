//! # Artifacts CLI
//!
//! A Rust-based CLI for generating, serializing, and deserializing secrets
//! (artifacts) for NixOS configurations. This tool manages artifacts through
//! configurable backends with interactive user prompts for secret generation.
//!
//! ## Core Functionality
//!
//! The CLI launches an interactive terminal UI that drives the full
//! check / generate / serialize lifecycle for every artifact declared in the
//! flake. It is the only mode of operation; there are no subcommands.
//!
//! ## Architecture
//!
//! The codebase is organized into several key modules:
//!
//! - [`app`]: Pure functional core using Elm Architecture pattern.
//!   Contains Model (state), Message (events), Update (transitions), and Effect
//!   (side effects) components.
//!
//! - [`backend`]: Backend operations for script execution,
//!   serialization/deserialization, and temporary file management.
//!
//! - [`cli`]: Command-line interface, argument parsing, and command
//!   orchestration.
//!
//! - [`config`]: Configuration parsing for `backend.toml` and
//!   flake.nix evaluation to extract artifact definitions.
//!
//! - [`tui`]: Terminal UI implementation using `ratatui`, including
//!   event handling, views, and runtime loop.
//!
//! - [`logging`]: Optional file-based logging infrastructure.
//!
//! ## Feature Flags
//!
//! The crate supports the following Cargo features:
//!
//! - **`logging`**: Enables file-based logging via the custom Logger.
//!   When enabled, logging macros ([`log_error!`], [`log_warn!`], [`log_info!`], [`log_debug!`])
//!   write to log files. When disabled, these macros compile to nothing
//!   (zero-cost abstraction).
//!
//! ## Example Usage
//!
//! ```bash
//! # Launch interactive TUI in the current flake directory
//! artifacts
//!
//! # Point the TUI at a different flake directory
//! artifacts /path/to/flake
//!
//! # Enable file logging for debugging
//! artifacts --log-file /tmp/artifacts.log --log-level debug
//! ```
//!
//! ## Configuration
//!
//! The CLI expects:
//! - A `flake.nix` in the current directory containing `nixosConfigurations`
//!   and/or `homeConfigurations` with artifact definitions
//! - A `backend.toml` defining serialization backends (agenix, sops-nix, etc.)
//!
//! ## Key Types
//!
//! - [`Logger`]: File-based logging (when `logging` feature enabled)
//!
// Crate-wide lint configuration: Only truly global style choices remain here.
// All other allows are scoped to specific items for better lint coverage.
//
// - module_name_repetitions: Naming convention choice (e.g., app::app_model)
// - must_use_candidate: Too noisy for internal APIs
// - missing_errors_doc/missing_panics_doc: To be addressed in Phase 21 (Documentation)
#![allow(
    clippy::module_name_repetitions,
    clippy::must_use_candidate,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc
)]

pub mod app;
pub mod backend;
pub mod logging;
#[macro_use]
pub mod macros;
pub mod cli;
pub mod config;
pub mod tui;

// Logging macros (log_error!, log_warn!, log_info!, log_debug!) are automatically exported
// at the crate root via #[macro_export] in src/logging.rs.
// They are feature-gated - when "logging" feature is disabled, they compile to nothing.

/// File-based logging infrastructure (requires `logging` feature).
///
/// See the [`logging`] module for details on log file
/// management and the Logger API.
pub use crate::logging::Logger;
