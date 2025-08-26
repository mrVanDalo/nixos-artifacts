use serde::{Deserialize, Serialize};

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
    #[serde(rename = "type")]
    pub kind: Option<String>, // hidden | line | multiline
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactDef {
    /// artifact name
    pub name: String,
    /// is artifact shared across machines
    /// (not implemented yet)
    pub shared: Option<bool>,
    /// files to be created
    #[serde(default)]
    pub files: Vec<FileDef>,
    /// prompts to be asked
    #[serde(default)]
    pub prompts: Vec<PromptDef>,
    /// generator script to be run to generate secrets
    pub generator: String,
    /// serialization script to be run to serialize secrets
    pub serialization: Option<String>, // backend reference name
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MakeConfig {
    #[serde(default)]
    pub machines: Vec<ArtifactDef>,
}
