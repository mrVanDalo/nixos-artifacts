use std::path::{Path, PathBuf};

pub mod generator;
pub mod prompt;
pub mod temp_dir;

pub(crate) fn resolve_path(base_dir: &Path, relative_path: &str) -> PathBuf {
    let path = Path::new(relative_path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base_dir.join(path)
    }
}
