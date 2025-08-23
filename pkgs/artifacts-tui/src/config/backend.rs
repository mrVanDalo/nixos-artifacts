use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BackendSettings(pub HashMap<String, serde_json::Value>);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendEntry {
    pub deserialize: String,
    pub serialize: String,
    #[serde(default)]
    pub settings: BackendSettings,
}

/// Maps backend name to its configuration
pub type BackendConfig = HashMap<String, BackendEntry>;
