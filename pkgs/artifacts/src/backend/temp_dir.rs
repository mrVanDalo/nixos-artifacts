//! Temporary directory management with automatic cleanup.
//!
//! This module provides `TempDirGuard`, a simple RAII guard for temporary
//! directories that ensures cleanup on drop. Unlike `TempFile` in tempfile.rs,
//! this guard focuses specifically on directory creation and cleanup without
//! additional features.
//!
//! # Use Cases
//!
//! - Simple temporary directory creation
//! - Scenarios where only path access is needed
//! - Lightweight alternative to TempFile when only directory cleanup is required
//!
//! # Comparison with tempfile.rs
//!
//! - `TempDirGuard`: Minimal, only tracks path and cleans up on drop
//! - `TempFile`: Full-featured, tracks metadata (size, created time), implements Deref
//!
//! Prefer `TempFile::new_dir()` in tempfile.rs for most use cases.

use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

/// A guard to ensure a temporary directory is removed on drop.
///
/// This struct wraps a temporary directory path and automatically
/// deletes the directory (and all its contents) when the guard is dropped.
///
/// # Drop Behavior
///
/// On drop, recursively deletes the directory using `fs::remove_dir_all`.
/// Errors are silently ignored to prevent panics during stack unwinding.
///
/// # Example
///
/// ```rust,ignore
/// {
///     let guard = create_temp_dir(Some("my-temp"))?;
///     // Use guard.path_buf to access the directory
///     let file_path = guard.path_buf.join("data.txt");
///     fs::write(&file_path, b"content")?;
/// } // directory automatically deleted here
/// ```
pub struct TempDirGuard {
    /// Path to the temporary directory
    pub path_buf: PathBuf,
}

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        // Best-effort cleanup; ignore errors so Drop never panics
        let _ = fs::remove_dir_all(&self.path_buf);
    }
}

/// Create a temporary directory under the system temp dir.
/// If `subfolder` is provided, it will be appended to the temp path.
/// Returns a guard that will remove the directory on drop.
// todo : it seems /tmp/<subfolder> is always the scenario, but we want something like tempdir creates
pub fn create_temp_dir(subfolder: Option<&str>) -> Result<TempDirGuard> {
    let mut directory = std::env::temp_dir();
    if let Some(sub) = subfolder {
        directory = directory.join(sub);
    }
    fs::create_dir_all(&directory).context("creating tmp directory")?;
    Ok(TempDirGuard {
        path_buf: directory,
    })
}
