use crate::config::make::ArtifactDef;
use log::debug;
use std::path::{Path, PathBuf};

pub fn print_files(artifact: &ArtifactDef, make_base: &Path) {
    if artifact.files.is_empty() {
        return;
    }
    debug!("    files to produce -> {} files", artifact.files.len());
    for f in &artifact.files {
        let resolved = resolve_path(make_base, &f.path);
        debug!(
            "      - {} => {}{}{}",
            f.name,
            resolved.display(),
            f.owner
                .as_ref()
                .map(|o| format!(" owner={}", o))
                .unwrap_or_default(),
            f.group
                .as_ref()
                .map(|g| format!(" group={}", g))
                .unwrap_or_default(),
        );
    }
}

pub(crate) fn resolve_path(base_dir: &Path, relative_path: &str) -> PathBuf {
    let path = Path::new(relative_path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base_dir.join(path)
    }
}
