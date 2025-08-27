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

/// Create a temporary directory under the system temp dir.
/// If `subfolder` is provided, it will be appended to the temp path.
/// Returns a guard that will remove the directory on drop.
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
