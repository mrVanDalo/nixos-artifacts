//! # Artifacts CLI
//!
//! A Rust-based CLI for generating, serializing, and deserializing secrets
//! (artifacts) for NixOS configurations. This tool manages artifacts through
//! configurable backends with interactive user prompts for secret generation.
//!
//! ## Core Functionality
//!
//! The CLI provides three main operations:
//!
//! - **TUI Mode** (`artifacts` or `artifacts tui`): Interactive terminal UI for
//!   managing artifacts with real-time status updates
//! - **Generate** (`artifacts generate`): Headless artifact generation
//! - **List** (`artifacts list`): Display all configured artifacts
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
//! # Launch interactive TUI
//! artifacts tui
//!
//! # Generate all artifacts that need regeneration
//! artifacts generate
//!
//! # List all configured artifacts
//! artifacts list
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
//! - `EffectHandler`: Bridges TUI foreground with background task execution
//! - [`Logger`]: File-based logging (when `logging` feature enabled)
//!
//! # Pedantic lints we intentionally allow:
// - must_use_candidate: Too noisy for this codebase - internal APIs don't need must_use
// - module_name_repetitions: Naming convention choice (e.g., app::model)
// - similar_names: Too strict on variable naming
// - missing_errors_doc: To be addressed in Phase 21 (Documentation)
// - missing_panics_doc: To be addressed in Phase 21 (Documentation)
// - return_self_not_must_use: Builder patterns often intentionally allow discarding
// - missing_const_for_fn: Not all functions need to be const
// - use_self: Good practice but too many instances to fix now
// - doc_markdown: Many items to fix, will be addressed incrementally
// - unreadable_literal: Most literals are readable already
// - unnested_or_patterns: Already fixed key instances
// - uninlined_format_args: Style preference, inline variables not always clearer
// - too_many_lines: Function length limits are too strict for complex state machines
// - redundant_closure: Style preference, closures are often clearer
// - unnecessary_wraps: API design choice - consistent Result types
// - map_unwrap_or: Style preference, often matches are clearer
// - from_over_into: Implementation detail, from and into are equivalent
// - ptr_as_ptr: Too strict for safe casts
// - borrow_as_ptr: Style preference
// - explicit_into_iter_loop: Style preference
// - filter_map_next: Style preference, find_map is equivalent
// - option_if_let_else: Style preference, if-let is clearer sometimes
// - map_flatten: Style preference, flatten is clearer sometimes
// - implicit_hasher: API design choice
// - explicit_iter_loop: Style preference
// - explicit_deref_methods: Style preference, *deref is clearer
// - single_match_else: Style preference, match is often clearer
// - match_wildcard_for_single_variants: Pattern completeness is important
// - option_option: API design choice
// - unwrap_used: Too strict - unwrap acceptable in some contexts
// - wildcard_imports: Style preference for convenience
// - cloned_instead_of_copied: Style preference, sometimes clearer
// - redundant_closure_for_method_calls: Style preference
// - items_after_statements: Style preference for test organization
// - wildcard_enum_match_arm: Pattern completeness is important
// - let_underscore_drop: Style preference
// - manual_let_else: Style preference, if-let is often clearer
// - bool_not_bitor: Style preference
// - needless_pass_by_value: API design choice
// - default_trait_access: Style preference
// - needless_borrowed_reference: Style preference
// - ref_option_ref: API design choice
// - string_add: Style preference for String concatenation
// - unused_async: API design for future async usage
// - cast_possible_truncation: Often intentional casts
// - cast_lossless: Too strict
// - manual_unwrap_or: Style preference
// - manual_unwrap_or_default: Style preference
// - format_push_string: Style preference for building strings
// - expect_fun_call: Style preference
// - if_not_else: Style preference
// - match_single_binding: Style preference
// - redundant_clone: Sometimes clones are clearer
// - debug_assert_with_mut_call: Rare edge case
// - needless_borrow: Style preference
// - single_char_pattern: Style preference
// - match_same_arms: Sometimes duplicate arms are clearer
// - duration_subsec: Rare edge case
// - significant_drop_tightening: Style preference
// - manual_non_exhaustive: API design choice
// - debug_non_exhaustive: Style preference
#![allow(
    clippy::must_use_candidate,
    clippy::module_name_repetitions,
    clippy::similar_names,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::return_self_not_must_use,
    clippy::missing_const_for_fn,
    clippy::use_self,
    clippy::doc_markdown,
    clippy::uninlined_format_args,
    clippy::too_many_lines,
    clippy::redundant_closure,
    clippy::unnecessary_wraps,
    clippy::map_unwrap_or,
    clippy::from_over_into,
    clippy::ptr_as_ptr,
    clippy::borrow_as_ptr,
    clippy::unwrap_used,
    let_underscore_drop,
    clippy::match_same_arms,
    clippy::duration_subsec,
    clippy::significant_drop_tightening,
    clippy::expect_fun_call,
    clippy::manual_non_exhaustive,
    clippy::needless_pass_by_value,
    clippy::default_trait_access,
    clippy::needless_borrowed_reference,
    clippy::ref_option_ref,
    clippy::string_add,
    clippy::unused_async,
    clippy::cast_possible_truncation,
    clippy::cast_lossless,
    clippy::manual_unwrap_or,
    clippy::manual_unwrap_or_default,
    clippy::format_push_string,
    clippy::if_not_else,
    clippy::match_single_binding,
    clippy::redundant_clone,
    clippy::debug_assert_with_mut_call,
    clippy::needless_borrow,
    clippy::single_char_pattern,
    clippy::cloned_instead_of_copied,
    clippy::redundant_closure_for_method_calls,
    clippy::items_after_statements,
    clippy::wildcard_enum_match_arm,
    clippy::manual_let_else,
    clippy::doc_lazy_continuation,
    clippy::blocks_in_conditions,
    clippy::module_inception,
)]

// Allow warnings for lints that are too strict even for pedantic
#![allow(
    clippy::unwrap_or_default,
    clippy::single_match,
    clippy::bind_instead_of_map,
    clippy::match_bool,
    clippy::needless_lifetimes,
    clippy::wrong_self_convention,
)]

pub mod app;
pub mod backend;
pub mod effect_handler;
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
