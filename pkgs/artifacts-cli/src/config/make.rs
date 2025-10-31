use anyhow::Context;
use log::debug;
use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize};
use serde_json::{Value, from_str as json_from_str, to_string_pretty};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDef {
    pub name: String,
    pub path: Option<String>,
    pub owner: Option<String>,
    pub group: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptDef {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ArtifactDef {
    /// artifact name
    pub name: String,
    /// is artifact shared across machines
    /// (not implemented yet)
    pub shared: bool,
    /// files to be created, keyed by file name
    pub files: BTreeMap<String, FileDef>,
    /// prompts to be asked, keyed by prompt name
    pub prompts: BTreeMap<String, PromptDef>,
    /// generator script to be run to generate secrets
    pub generator: String,
    /// serialization script to be run to serialize secrets
    pub serialization: String, // backend reference name
}

impl<'de> Deserialize<'de> for ArtifactDef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct ArtifactDefHelper {
            name: Option<String>,
            shared: Option<bool>,
            #[serde(default)]
            files: BTreeMap<String, FileDef>,
            #[serde(default)]
            prompts: BTreeMap<String, PromptDef>,
            generator: Option<String>,
            serialization: Option<String>,
        }

        let helper = ArtifactDefHelper::deserialize(deserializer)?;
        let name = match helper.name {
            Some(n) if !n.is_empty() => n,
            _ => return Err(de::Error::custom("name must be set")),
        };
        let shared = helper.shared.unwrap_or(false);
        let generator = match helper.generator {
            Some(g) if !g.is_empty() => g,
            _ => return Err(de::Error::custom("generator must be set")),
        };
        let serialization = match helper.serialization {
            Some(s) if !s.is_empty() => s,
            _ => return Err(de::Error::custom("serialization must be set")),
        };
        Ok(ArtifactDef {
            name,
            shared,
            files: helper.files,
            prompts: helper.prompts,
            generator,
            serialization,
        })
    }
}

pub struct MakeConfiguration {
    // nixos: machine-name -> (artifact-name -> artifact)
    pub nixos_map: BTreeMap<String, BTreeMap<String, ArtifactDef>>,
    // home: user-name -> (artifact-name -> artifact)
    pub home_map: BTreeMap<String, BTreeMap<String, ArtifactDef>>,
    // nixos: machine-name -> (backend-name -> backend-config map)
    pub nixos_config: BTreeMap<String, BTreeMap<String, BTreeMap<String, Value>>>,
    // home: user-name -> (backend-name -> backend-config map)
    pub home_config: BTreeMap<String, BTreeMap<String, BTreeMap<String, Value>>>,
    pub make_base: PathBuf,
    pub make_json: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MachineArtifacts {
    machine: String,
    #[serde(default)]
    artifacts: BTreeMap<String, ArtifactDef>,
    /// per-machine backend configuration: backend name -> { setting -> value(s) }
    #[serde(default)]
    config: BTreeMap<String, BTreeMap<String, Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HomeArtifacts {
    user: String,
    #[serde(default)]
    artifacts: BTreeMap<String, ArtifactDef>,
    /// per-user backend configuration: backend name -> { setting -> value(s) }
    #[serde(default)]
    config: BTreeMap<String, BTreeMap<String, Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MakeRoot {
    #[serde(default)]
    nixos: Vec<MachineArtifacts>,
    #[serde(default)]
    home: Vec<HomeArtifacts>,
}

impl MakeConfiguration {
    pub(crate) fn read_make_config(make_json: &Path) -> anyhow::Result<MakeConfiguration> {
        let make_text = fs::read_to_string(make_json)
            .with_context(|| format!("reading make config {}", make_json.display()))?;

        let pretty = match json_from_str::<Value>(&make_text) {
            Ok(v) => to_string_pretty(&v).unwrap_or_else(|_| make_text.clone()),
            Err(_) => make_text.clone(),
        };
        debug!("make config (pretty):\n{}", pretty);

        let root: MakeRoot = json_from_str(&make_text)
            .with_context(|| format!("parsing make config {}", make_json.display()))?;

        let mut nixos_map: BTreeMap<String, BTreeMap<String, ArtifactDef>> = BTreeMap::new();
        let mut home_map: BTreeMap<String, BTreeMap<String, ArtifactDef>> = BTreeMap::new();
        let mut nixos_config: BTreeMap<String, BTreeMap<String, BTreeMap<String, Value>>> =
            BTreeMap::new();
        let mut home_config: BTreeMap<String, BTreeMap<String, BTreeMap<String, Value>>> =
            BTreeMap::new();

        for entry in root.nixos {
            let machine_map = nixos_map.entry(entry.machine.clone()).or_default();
            for (art_name, art) in entry.artifacts {
                machine_map.insert(art_name, art);
            }
            if !entry.config.is_empty() {
                nixos_config.insert(entry.machine, entry.config);
            }
        }
        for entry in root.home {
            let user_map = home_map.entry(entry.user.clone()).or_default();
            for (art_name, art) in entry.artifacts {
                user_map.insert(art_name, art);
            }
            if !entry.config.is_empty() {
                home_config.insert(entry.user, entry.config);
            }
        }

        let make_base = make_json
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."));
        Ok(MakeConfiguration {
            nixos_map,
            home_map,
            nixos_config,
            home_config,
            make_base,
            make_json: make_json.to_path_buf(),
        })
    }
}

impl MakeConfiguration {
    pub fn get_backend_config_for(
        &self,
        target_name: &str,
        backend_name: &str,
    ) -> Option<&BTreeMap<String, Value>> {
        self.nixos_config
            .get(target_name)
            .and_then(|m| m.get(backend_name))
            .or_else(|| {
                self.home_config
                    .get(target_name)
                    .and_then(|m| m.get(backend_name))
            })
    }
}
