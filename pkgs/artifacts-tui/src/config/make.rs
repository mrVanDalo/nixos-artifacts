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
    pub name: String,
    pub shared: Option<bool>,
    #[serde(default)]
    pub files: Vec<FileDef>,
    #[serde(default)]
    pub prompts: Vec<PromptDef>,
    pub generator: Option<String>,
    pub serialization: Option<String>, // backend reference name
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MakeConfig {
    #[serde(default)]
    pub machines: Vec<ArtifactDef>,
}
