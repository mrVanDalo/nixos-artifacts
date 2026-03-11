//! Helper utilities for backend script execution and path handling.
//!
//! This module provides common utility functions used by the backend module:
//! - Path resolution for relative script paths
//! - Shell escaping for safe command construction
//! - Hash generation for deterministic temporary filenames
//! - Backend script validation (executable checks)
//!
//! # Safety
//!
//! All path operations handle both absolute and relative paths correctly.
//! Shell escaping functions properly quote special characters to prevent
//! injection attacks when constructing shell commands.

use anyhow::{Result, bail};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

/// Validate that a backend script exists, is a file, and is executable.
///
/// Performs comprehensive validation of a backend script:
/// 1. Checks that the script file exists
/// 2. Verifies it's a regular file (not a directory)
/// 3. Ensures the file has execute permissions
///
/// # Arguments
///
/// * `backend_name` - Name of the backend (for error messages)
/// * `step_name` - Name of the step (e.g., "serialize", "check_serialization")
/// * `base_path` - Base directory for resolving relative paths
/// * `script_path` - Path to the script (relative to base_path or absolute)
///
/// # Returns
///
/// Returns the canonicalized (absolute) path to the script if validation passes.
///
/// # Errors
///
/// Returns an error if:
/// - The script file does not exist
/// - The path points to a directory instead of a file
/// - The file is not executable
/// - The path cannot be resolved
///
/// # Example
///
/// ```rust,ignore
/// let script_path = validate_backend_script(
///     "agenix",
///     "serialize",
///     Path::new("/project/backends"),
///     "agenix_serialize.sh",
/// )?;
/// // script_path is now absolute: /project/backends/agenix_serialize.sh
/// ```
pub fn validate_backend_script(
    backend_name: &str,
    step_name: &str,
    base_path: &Path,
    script_path: &str,
) -> Result<PathBuf> {
    let resolved = resolve_path(base_path, script_path);

    if !resolved.exists() {
        bail!(
            "backend '{}' step '{}': script '{}' does not exist",
            backend_name,
            step_name,
            resolved.display()
        );
    }

    if !resolved.is_file() {
        bail!(
            "backend '{}' step '{}': '{}' is not a file",
            backend_name,
            step_name,
            resolved.display()
        );
    }

    // Check if file is executable
    let metadata = std::fs::metadata(&resolved)?;
    let permissions = metadata.permissions();
    if permissions.mode() & 0o111 == 0 {
        bail!(
            "backend '{}' step '{}': '{}' is not executable",
            backend_name,
            step_name,
            resolved.display()
        );
    }

    // Return canonicalized path for cleaner logs
    Ok(std::fs::canonicalize(&resolved).unwrap_or(resolved))
}

/// Compute a 64-bit FNV-1a hash of a string.
///
/// This hash function is used to generate deterministic filenames based on
/// the 'out' path. The same input string will always produce the same hash,
/// which keeps test snapshots stable across runs.
///
/// # Algorithm
///
/// Uses the FNV-1a (Fowler-Noll-Vo) hash algorithm:
/// - Offset basis: 0xcbf2_9ce4_8422_2325
/// - Prime: 0x0000_0100_0000_01B3
///
/// # Arguments
///
/// * `s` - The string to hash
///
/// # Returns
///
/// Returns a 64-bit unsigned hash value
///
/// # Example
///
/// ```rust,ignore
/// let hash = fnv1a64("/tmp/artifact-out");
/// // hash is deterministic for the same input
/// ```
pub fn fnv1a64(s: &str) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325; // FNV offset basis
    const PRIME: u64 = 0x0000_0100_0000_01B3; // FNV prime
    for b in s.as_bytes() {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}

/// Resolve a path relative to a base directory.
///
/// If the path is already absolute, it's returned as-is. Otherwise, it's
/// resolved relative to the provided base directory.
///
/// # Arguments
///
/// * `base_dir` - The base directory for relative path resolution
/// * `relative_path` - The path to resolve (can be absolute or relative)
///
/// # Returns
///
/// Returns a `PathBuf` representing the resolved path
///
/// # Example
///
/// ```rust,ignore
/// // Relative path - resolved against base_dir
/// let path = resolve_path(
///     Path::new("/project"),
///     "backends/agenix.sh",
/// );
/// assert_eq!(path, PathBuf::from("/project/backends/agenix.sh"));
///
/// // Absolute path - returned as-is
/// let path = resolve_path(
///     Path::new("/project"),
///     "/etc/nixos/config.nix",
/// );
/// assert_eq!(path, PathBuf::from("/etc/nixos/config.nix"));
/// ```
pub(crate) fn resolve_path(base_dir: &Path, relative_path: &str) -> PathBuf {
    let path = Path::new(relative_path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base_dir.join(path)
    }
}

#[rustfmt::skip]
pub fn pretty_print_shell_escape(input: &str) -> String {
    let needs_quotes = input.is_empty() || input.chars().any(|character| { character.is_whitespace() || matches!( character, '\'' | '"' | '\\' | '$' | '&' | '|' | ';' | '<' | '>' | '(' | ')' | '[' | ']' | '{' | '}' ) });
    if needs_quotes {
        format!("'{}'", escape_single_quoted(input))
    } else {
        input.to_string()
    }
}

/// Escape a string for safe use within single-quoted shell arguments.
///
/// Single quotes in shell are literal - except that a single quote itself
/// cannot appear inside single quotes. This function escapes single quotes
/// by ending the quoted string, adding an escaped quote, and starting a
/// new quoted string.
///
/// # Arguments
///
/// * `input` - The string to escape
///
/// # Returns
///
/// Returns the escaped string suitable for use within single quotes
///
/// # Example
///
/// ```rust,ignore
/// let escaped = escape_single_quoted("it's a test");
/// // Returns: it'\''s a test
/// // In shell: 'it'\''s a test' produces: it's a test
/// ```
pub fn escape_single_quoted(input: &str) -> String {
    input.replace('\'', "'\\''")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use tempfile::TempDir;

    #[test]
    fn test_validate_backend_script_not_exists() {
        let temp_dir = TempDir::new().unwrap();
        let result = validate_backend_script(
            "test-backend",
            "serialize",
            temp_dir.path(),
            "nonexistent.sh",
        );

        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("backend 'test-backend' step 'serialize'"),
            "error should mention backend and step: {}",
            err
        );
        assert!(
            err.contains("does not exist"),
            "error should mention 'does not exist': {}",
            err
        );
    }

    #[test]
    fn test_validate_backend_script_is_directory() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path().join("not-a-file");
        fs::create_dir(&dir_path).unwrap();

        let result = validate_backend_script(
            "test-backend",
            "check_serialization",
            temp_dir.path(),
            "not-a-file",
        );

        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("backend 'test-backend' step 'check_serialization'"),
            "error should mention backend and step: {}",
            err
        );
        assert!(
            err.contains("is not a file"),
            "error should mention 'is not a file': {}",
            err
        );
    }

    #[test]
    fn test_validate_backend_script_not_executable() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("script.sh");
        File::create(&script_path).unwrap();

        // Ensure file is NOT executable (mode 0o644)
        fs::set_permissions(&script_path, fs::Permissions::from_mode(0o644)).unwrap();

        let result = validate_backend_script("agenix", "deserialize", temp_dir.path(), "script.sh");

        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("backend 'agenix' step 'deserialize'"),
            "error should mention backend and step: {}",
            err
        );
        assert!(
            err.contains("is not executable"),
            "error should mention 'is not executable': {}",
            err
        );
    }

    #[test]
    fn test_validate_backend_script_success() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("valid.sh");
        File::create(&script_path).unwrap();

        // Make file executable (mode 0o755)
        fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755)).unwrap();

        let result = validate_backend_script("sops", "serialize", temp_dir.path(), "valid.sh");

        assert!(
            result.is_ok(),
            "should succeed for valid executable: {:?}",
            result
        );
        let path = result.unwrap();
        assert!(path.ends_with("valid.sh"));
    }

    #[test]
    fn test_validate_backend_script_absolute_path() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("absolute.sh");
        File::create(&script_path).unwrap();
        fs::set_permissions(&script_path, fs::Permissions::from_mode(0o755)).unwrap();

        // Use absolute path instead of relative
        let result = validate_backend_script(
            "test",
            "serialize",
            Path::new("/some/other/base"),
            script_path.to_str().unwrap(),
        );

        assert!(
            result.is_ok(),
            "should work with absolute path: {:?}",
            result
        );
    }
}
