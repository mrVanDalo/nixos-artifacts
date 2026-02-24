//! Utility macros for the artifacts-cli crate.
//!
//! This module provides macros for common patterns used throughout the codebase:
//!
//! - `string_vec!` - Convert string literals to a `Vec<String>`
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

// Allow the file to compile even if not directly referenced besides the macro export
#[allow(dead_code)]
const _MACROS_RS: () = ();
