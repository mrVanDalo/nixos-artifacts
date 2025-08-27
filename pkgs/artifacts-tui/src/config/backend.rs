use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

/// Container combining backend configuration and its base path
#[derive(Debug, Clone)]
pub struct BackendConfiguration {
    pub config: HashMap<String, BackendEntry>,
    pub base_path: PathBuf,
    pub backend_toml: PathBuf,
}

impl BackendConfiguration {
    pub fn read_backend_config(backend_toml: &Path) -> Result<BackendConfiguration> {
        let backend_text = fs::read_to_string(backend_toml)
            .with_context(|| format!("reading backend config {}", backend_toml.display()))?;
        let backend_config: HashMap<String, BackendEntry> = toml::from_str(&backend_text)
            .with_context(|| format!("parsing backend config {}", backend_toml.display()))?;
        let backend_base = backend_toml
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        Ok(BackendConfiguration {
            config: backend_config,
            base_path: backend_base,
            backend_toml: backend_toml.to_path_buf(),
        })
    }

    pub(crate) fn get_backend(&self, backend_name: &String) -> Result<BackendEntry> {
        match self.config.get(backend_name) {
            Some(entry) => Ok(entry.clone()),
            None => Err(anyhow::anyhow!(
                "backend '{}' not found in {}",
                backend_name,
                self.backend_toml.display()
            )),
        }
    }
}
