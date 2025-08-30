use anyhow::Context;
use serde::{Deserialize, Serialize};
use serde_json::from_str as json_from_str;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDef {
    pub name: String,
    pub path: String,
    pub owner: Option<String>,
    pub group: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptDef {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactDef {
    /// artifact name
    pub name: String,
    /// is artifact shared across machines
    /// (not implemented yet)
    pub shared: Option<bool>,
    /// files to be created, keyed by file name
    #[serde(default)]
    pub files: BTreeMap<String, FileDef>,
    /// prompts to be asked, keyed by prompt name
    #[serde(default)]
    pub prompts: BTreeMap<String, PromptDef>,
    /// generator script to be run to generate secrets
    pub generator: String,
    /// serialization script to be run to serialize secrets
    pub serialization: String, // backend reference name
}

pub struct MakeConfiguration {
    // machine-name -> (artifact-name -> artifact)
    pub make_map: BTreeMap<String, BTreeMap<String, ArtifactDef>>,
    pub make_base: PathBuf,
    pub make_json: PathBuf,
}

impl MakeConfiguration {
    pub(crate) fn read_make_config(make_json: &Path) -> anyhow::Result<MakeConfiguration> {
        let make_text = fs::read_to_string(make_json)
            .with_context(|| format!("reading make config {}", make_json.display()))?;
        let make_map: BTreeMap<String, BTreeMap<String, ArtifactDef>> =
            json_from_str(&make_text)
                .with_context(|| format!("parsing make config {}", make_json.display()))?;
        let make_base = make_json
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        Ok(MakeConfiguration {
            make_map,
            make_base,
            make_json: make_json.to_path_buf(),
        })
    }
}
