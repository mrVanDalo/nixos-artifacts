use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

/// A guard to ensure a temporary directory is removed on drop
pub(crate) struct TempDirGuard {
    pub path_buf: PathBuf,
}

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        // Best-effort cleanup; ignore errors so Drop never panics
        let _ = fs::remove_dir_all(&self.path_buf);
    }
}

/// Manages temporary directories for the application.
/// It keeps track of all created temp bases and holds CleanupGuards to
/// ensure they are removed on drop.
pub struct TempManager {}

impl TempManager {
    pub(crate) fn new() -> Self {
        Self { /* fields */ }
    }
}

impl TempManager {
    pub fn create_temp_dir(&self, subfolder: Option<&str>) -> Result<TempDirGuard> {
        let mut directory = std::env::temp_dir();
        if let Some(sub) = subfolder {
            directory = directory.join(sub);
        }
        fs::create_dir_all(&directory).context("creating tmp directory")?;
        Ok(TempDirGuard {
            path_buf: directory,
        })
    }
}
