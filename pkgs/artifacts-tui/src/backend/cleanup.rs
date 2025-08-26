use std::fs;
use std::path::PathBuf;

pub(crate) struct CleanupGuard {
    base: PathBuf,
}

impl CleanupGuard {
    pub(crate) fn new(base: PathBuf) -> Self {
        Self { base }
    }
}

impl Drop for CleanupGuard {
    fn drop(&mut self) {
        // Best-effort cleanup; ignore errors so Drop never panics
        let _ = fs::remove_dir_all(&self.base);
    }
}
