use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

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

/// Maps backend name to its configuration
pub type BackendConfig = HashMap<String, BackendEntry>;

/// Container combining backend configuration and its base path
#[derive(Debug, Clone)]
pub struct BackendConfiguration {
    pub config: BackendConfig,
    pub base_path: PathBuf,
}
