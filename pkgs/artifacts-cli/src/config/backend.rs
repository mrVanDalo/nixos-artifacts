use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BackendSettings(pub HashMap<String, serde_json::Value>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendEntry {
    pub check_serialization: String,
    pub deserialize: String,
    pub serialize: String,
    #[serde(default)]
    pub settings: BackendSettings,
}

/// Intermediate structure for parsing TOML with optional includes
#[derive(Debug, Deserialize)]
struct BackendFileRaw {
    #[serde(default)]
    include: Vec<String>,
    #[serde(flatten)]
    backends: HashMap<String, BackendEntry>,
}

/// Container combining backend configuration and its base path
#[derive(Debug, Clone)]
pub struct BackendConfiguration {
    pub config: HashMap<String, BackendEntry>,
    pub base_path: PathBuf,
    pub backend_toml: PathBuf,
}

impl BackendConfiguration {
    pub fn read_backend_config(backend_toml: &Path) -> Result<BackendConfiguration> {
        let mut visited = HashSet::new();
        let config = Self::load_with_includes(backend_toml, &mut visited)?;

        let backend_base = backend_toml
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));

        Ok(BackendConfiguration {
            config,
            base_path: backend_base,
            backend_toml: backend_toml.to_path_buf(),
        })
    }

    fn load_with_includes(
        toml_path: &Path,
        visited: &mut HashSet<PathBuf>,
    ) -> Result<HashMap<String, BackendEntry>> {
        let canonical = toml_path
            .canonicalize()
            .with_context(|| format!("resolving path {}", toml_path.display()))?;

        if !visited.insert(canonical.clone()) {
            anyhow::bail!("circular include detected: {}", toml_path.display());
        }

        let text = fs::read_to_string(&canonical)
            .with_context(|| format!("reading backend config {}", toml_path.display()))?;

        let raw: BackendFileRaw = toml::from_str(&text)
            .with_context(|| format!("parsing backend config {}", toml_path.display()))?;

        let mut result = raw.backends;

        // Get the directory containing this file for relative path resolution
        let base_dir = canonical.parent().unwrap_or(Path::new("."));

        for include_path in raw.include {
            let resolved_path = base_dir.join(&include_path);
            let included = Self::load_with_includes(&resolved_path, visited)?;

            for (key, value) in included {
                if result.contains_key(&key) {
                    anyhow::bail!(
                        "duplicate backend '{}' found when including {} from {}",
                        key,
                        include_path,
                        toml_path.display()
                    );
                }
                result.insert(key, value);
            }
        }

        Ok(result)
    }

    pub(crate) fn get_backend(&self, backend_name: &String) -> Result<BackendEntry> {
        let backend = self.config.get(backend_name).with_context(|| {
            format!(
                "backend '{}' not found in {}",
                backend_name,
                self.backend_toml.display()
            )
        })?;
        Ok(backend.clone())
    }
}
