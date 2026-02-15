use anyhow::{Context, Result};
use std::fs;
use std::io::Write;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Type of temporary resource
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TempType {
    File,
    Directory,
}

/// A temporary file or directory that is automatically cleaned up on drop
pub struct TempFile {
    path: PathBuf,
    pub created: SystemTime,
    pub size: u64,
    pub temp_type: TempType,
}

impl TempFile {
    /// Create a new temporary file with the given content
    pub fn new_file_with_content(prefix: &str, content: &[u8]) -> Result<Self> {
        let temp_dir = std::env::temp_dir();
        let file_name = format!("{}_{}", prefix, std::process::id());
        let path = temp_dir.join(file_name);

        let mut file = fs::File::create(&path)
            .with_context(|| format!("failed to create temporary file at {:?}", path))?;
        file.write_all(content)
            .with_context(|| format!("failed to write content to temporary file at {:?}", path))?;

        let metadata = fs::metadata(&path)
            .with_context(|| format!("failed to read metadata for temporary file at {:?}", path))?;

        Ok(Self {
            path,
            created: metadata.created().unwrap_or_else(|_| SystemTime::now()),
            size: metadata.len(),
            temp_type: TempType::File,
        })
    }

    /// Create a new empty temporary file
    pub fn new_file(prefix: &str) -> Result<Self> {
        Self::new_file_with_content(prefix, &[])
    }

    /// Create a new temporary directory with a unique name.
    pub fn new_dir(prefix: &str) -> Result<Self> {
        let temp_dir = std::env::temp_dir();
        let dir_name = format!("{}_{}", prefix, std::process::id());
        let path = temp_dir.join(dir_name);

        fs::create_dir_all(&path)
            .with_context(|| format!("failed to create temporary directory at {:?}", path))?;

        let metadata = fs::metadata(&path).with_context(|| {
            format!(
                "failed to read metadata for temporary directory at {:?}",
                path
            )
        })?;

        Ok(Self {
            path,
            created: metadata.created().unwrap_or_else(|_| SystemTime::now()),
            size: 0,
            temp_type: TempType::Directory,
        })
    }

    /// Create a temporary directory with a specific name under /tmp.
    /// Use this when you need a predictable path (e.g., for passing to scripts).
    /// Note: This may conflict with concurrent runs - prefer `new_dir()` when possible.
    pub fn new_dir_with_name(name: &str) -> Result<Self> {
        let temp_dir = std::env::temp_dir();
        let path = temp_dir.join(name);

        fs::create_dir_all(&path)
            .with_context(|| format!("failed to create temporary directory at {:?}", path))?;

        let metadata = fs::metadata(&path).with_context(|| {
            format!(
                "failed to read metadata for temporary directory at {:?}",
                path
            )
        })?;

        Ok(Self {
            path,
            created: metadata.created().unwrap_or_else(|_| SystemTime::now()),
            size: 0,
            temp_type: TempType::Directory,
        })
    }

    /// Get the path to the temporary file or directory
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Get the path as a PathBuf clone
    pub fn path_buf(&self) -> PathBuf {
        self.path.clone()
    }

    /// Check if this is a file
    pub fn is_file(&self) -> bool {
        self.temp_type == TempType::File
    }

    /// Check if this is a directory
    pub fn is_dir(&self) -> bool {
        self.temp_type == TempType::Directory
    }

    /// Update the size for directories by calculating total size
    pub fn refresh_size(&mut self) -> Result<u64> {
        if self.is_dir() {
            let size = Self::calculate_dir_size(&self.path)?;
            self.size = size;
            Ok(size)
        } else {
            let metadata = fs::metadata(&self.path)
                .with_context(|| format!("failed to read metadata for {:?}", self.path))?;
            self.size = metadata.len();
            Ok(self.size)
        }
    }

    /// Calculate the total size of a directory recursively
    fn calculate_dir_size(path: &Path) -> Result<u64> {
        let mut total_size = 0u64;

        for entry in
            fs::read_dir(path).with_context(|| format!("failed to read directory {:?}", path))?
        {
            let entry = entry?;
            let metadata = entry.metadata()?;

            if metadata.is_dir() {
                total_size += Self::calculate_dir_size(&entry.path())?;
            } else {
                total_size += metadata.len();
            }
        }

        Ok(total_size)
    }

    /// Create a temporary directory at a specific path location
    /// The parent directory must exist.
    pub fn create_dir_at(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();

        fs::create_dir_all(&path)
            .with_context(|| format!("failed to create temporary directory at {:?}", path))?;

        let metadata = fs::metadata(&path).with_context(|| {
            format!(
                "failed to read metadata for temporary directory at {:?}",
                path
            )
        })?;

        Ok(Self {
            path,
            created: metadata.created().unwrap_or_else(|_| SystemTime::now()),
            size: 0,
            temp_type: TempType::Directory,
        })
    }

    /// Create a temporary file at a specific path location
    /// The parent directory must exist.
    pub fn create_file_at(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create parent directory for {:?}", path))?;
        }

        fs::File::create(&path)
            .with_context(|| format!("failed to create temporary file at {:?}", path))?;

        let metadata = fs::metadata(&path)
            .with_context(|| format!("failed to read metadata for temporary file at {:?}", path))?;

        Ok(Self {
            path,
            created: metadata.created().unwrap_or_else(|_| SystemTime::now()),
            size: metadata.len(),
            temp_type: TempType::File,
        })
    }

    /// Create a temporary file with the given content at a specific path
    pub fn create_file_at_with_content(path: impl AsRef<Path>, content: &[u8]) -> Result<Self> {
        let path = path.as_ref().to_path_buf();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create parent directory for {:?}", path))?;
        }

        let mut file = fs::File::create(&path)
            .with_context(|| format!("failed to create temporary file at {:?}", path))?;
        file.write_all(content)
            .with_context(|| format!("failed to write content to temporary file at {:?}", path))?;

        let metadata = fs::metadata(&path)
            .with_context(|| format!("failed to read metadata for temporary file at {:?}", path))?;

        Ok(Self {
            path,
            created: metadata.created().unwrap_or_else(|_| SystemTime::now()),
            size: metadata.len(),
            temp_type: TempType::File,
        })
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        match self.temp_type {
            TempType::File => {
                if let Err(e) = fs::remove_file(&self.path) {
                    eprintln!(
                        "Warning: Failed to remove temporary file {:?}: {}",
                        self.path, e
                    );
                }
            }
            TempType::Directory => {
                if let Err(e) = fs::remove_dir_all(&self.path) {
                    eprintln!(
                        "Warning: Failed to remove temporary directory {:?}: {}",
                        self.path, e
                    );
                }
            }
        }
    }
}

impl Deref for TempFile {
    type Target = Path;

    fn deref(&self) -> &Self::Target {
        &self.path
    }
}

impl AsRef<Path> for TempFile {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::Duration;

    #[test]
    fn test_temp_file_creation() {
        let temp_file = TempFile::new_file("test").unwrap();
        assert!(temp_file.is_file());
        assert!(temp_file.path().exists());

        let path = temp_file.path().to_path_buf();
        drop(temp_file);

        std::thread::sleep(Duration::from_millis(100));
        assert!(!path.exists());
    }

    #[test]
    fn test_temp_dir_creation() {
        let temp_dir = TempFile::new_dir("test").unwrap();
        assert!(temp_dir.is_dir());
        assert!(temp_dir.path().exists());

        let path = temp_dir.path().to_path_buf();
        drop(temp_dir);

        std::thread::sleep(Duration::from_millis(100));
        assert!(!path.exists());
    }

    #[test]
    fn test_temp_file_with_content() {
        let content = b"Hello, World!";
        let temp_file = TempFile::new_file_with_content("test", content).unwrap();

        assert_eq!(temp_file.size, content.len() as u64);

        let read_content = fs::read(temp_file.path()).unwrap();
        assert_eq!(read_content, content);
    }

    #[test]
    fn test_deref() {
        let temp_file = TempFile::new_file("test").unwrap();
        let _path_ref: &Path = &*temp_file;
    }

    #[test]
    fn test_as_ref() {
        let temp_file = TempFile::new_file("test").unwrap();
        let _path_ref: &Path = temp_file.as_ref();
    }
}
