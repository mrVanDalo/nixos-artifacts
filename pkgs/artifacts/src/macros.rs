//! Utility macros for the artifacts-cli crate.
//!
//! This module provides macros for common patterns used throughout the codebase:
//!
//! - `string_vec!` - Convert string literals to a `Vec<String>`
//! - `log_debug!`, `log_trace!`, `log_error!` - Feature-gated logging macros
//!
//! ## The `string_vec!` Macro
//!
//! Converts a list of string literals or expressions into a `Vec<String>`.
//! This is useful for building lists of file paths or command arguments.
//!
//! ```ignore
//! use artifacts::string_vec;
//!
//! let args = string_vec!["--help", "--verbose"];
//! assert_eq!(args, vec!["--help", "--verbose"]);
//! ```
//!
//! ## Feature-Gated Logging
//!
//! The logging macros are only active when the `"logging"` Cargo feature is enabled.
//! When disabled (the default), these macros compile to nothing, incurring zero
//! runtime cost.
//!
//! ```ignore
//! use artifacts::log_debug;
//!
//! // This only outputs when "logging" feature is enabled
//! log_debug!("Processing artifact {}", name);
//! ```

/// Converts expressions to a `Vec<String>`.
///
/// This macro evaluates each expression and calls `.to_string()` on it,
/// then collects them into a vector. Useful for building command arguments
/// or lists of file paths from string literals.
///
/// # Examples
///
/// ```
/// use artifacts::string_vec;
///
/// let names = string_vec!["alice", "bob"];
/// assert_eq!(names, vec!["alice", "bob"]);
///
/// // Works with variables too
/// let user = "charlie";
/// let names = string_vec!["alice", user];
/// ```
#[macro_export]
macro_rules! string_vec {
    ($($x:expr),* $(,)?) => {
        vec![$($x.to_string()),*]
    };
}

/// Log a debug message (feature-gated).
///
/// When the `"logging"` feature is enabled, this delegates to [`log::debug!`].
/// When disabled, this macro expands to nothing (zero-cost abstraction).
///
/// # Examples
///
/// ```ignore
/// log_debug!("Processing {} files", file_count);
/// log_debug!("Artifact {} ready", artifact_name);
/// ```

#[cfg(feature = "logging")]
#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {
        log::debug!($($arg)*)
    };
}

#[cfg(not(feature = "logging"))]
#[macro_export]
#[doc(hidden)]
macro_rules! log_debug {
    ($($arg:tt)*) => {};
}

/// Log a trace message (feature-gated).
///
/// When the `"logging"` feature is enabled, this delegates to [`log::trace!`].
/// When disabled, this macro expands to nothing (zero-cost abstraction).
///
/// # Examples
///
/// ```ignore
/// log_trace!("Entering function {}", "process_artifact");
/// ```
#[cfg(feature = "logging")]
#[macro_export]
macro_rules! log_trace {
    ($($arg:tt)*) => {
        log::trace!($($arg)*)
    };
}

#[cfg(not(feature = "logging"))]
#[macro_export]
#[doc(hidden)]
macro_rules! log_trace {
    ($($arg:tt)*) => {};
}

/// Log an error message (feature-gated).
///
/// When the `"logging"` feature is enabled, this delegates to [`log::error!`].
/// When disabled, this macro expands to nothing (zero-cost abstraction).
///
/// # Examples
///
/// ```ignore
/// log_error!("Failed to generate artifact: {}", err);
/// ```
#[cfg(feature = "logging")]
#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {
        log::error!($($arg)*)
    };
}

#[cfg(not(feature = "logging"))]
#[macro_export]
#[doc(hidden)]
macro_rules! log_error {
    ($($arg:tt)*) => {};
}

// Allow the file to compile even if not directly referenced besides the macro export
#[allow(dead_code)]
const _MACROS_RS: () = ();
