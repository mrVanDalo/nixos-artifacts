use anyhow::Context;
use log::trace;
use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize};
use serde_json::{from_str as json_from_str, to_string_pretty, Value};
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

/// Type of target that defines an artifact
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum TargetType {
    Nixos,
    HomeManager,
}

/// Source of a generator - which target defines it
#[derive(Debug, Clone, Serialize)]
pub struct GeneratorSource {
    /// machine name or user identifier
    pub target: String,
    /// type of target
    pub target_type: TargetType,
}

/// A unique generator script with its sources
#[derive(Debug, Clone, Serialize)]
pub struct GeneratorInfo {
    /// generator script path
    pub path: String,
    /// targets that use this generator
    pub sources: Vec<GeneratorSource>,
}

/// Aggregated information about a shared artifact across all targets
#[derive(Debug, Clone, Serialize)]
pub struct SharedArtifactInfo {
    /// artifact name
    pub artifact_name: String,
    /// unique generators with their sources
    pub generators: Vec<GeneratorInfo>,
    /// NixOS machine names that use this artifact
    pub nixos_targets: Vec<String>,
    /// Home-manager user identifiers that use this artifact
    pub home_targets: Vec<String>,
    /// serialization backend name
    pub backend_name: String,
    /// prompts collected from first definition (shared artifacts should have identical prompts)
    pub prompts: BTreeMap<String, PromptDef>,
    /// files collected from first definition (shared artifacts should have identical files)
    pub files: BTreeMap<String, FileDef>,
}

#[derive(Clone)]
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
    pub fn read_make_config(make_json: &Path) -> anyhow::Result<MakeConfiguration> {
        let make_text = fs::read_to_string(make_json)
            .with_context(|| format!("reading make config {}", make_json.display()))?;

        let pretty = match json_from_str::<Value>(&make_text) {
            Ok(v) => to_string_pretty(&v).unwrap_or_else(|_| make_text.clone()),
            Err(_) => make_text.clone(),
        };
        trace!("make config (pretty):\n{}", pretty);

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
            for (artifact_name, artifact) in entry.artifacts {
                machine_map.insert(artifact_name, artifact);
            }
            if !entry.config.is_empty() {
                nixos_config.insert(entry.machine, entry.config);
            }
        }
        for entry in root.home {
            let user_map = home_map.entry(entry.user.clone()).or_default();
            for (artifact_name, artifact) in entry.artifacts {
                user_map.insert(artifact_name, artifact);
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

    /// Aggregate all shared artifacts across machines and home-manager users.
    /// Returns a map from artifact name to SharedArtifactInfo.
    pub fn get_shared_artifacts(&self) -> BTreeMap<String, SharedArtifactInfo> {
        // Collect all shared artifacts by name
        // Structure: artifact_name -> Vec<(target, target_type, artifact)>
        let mut shared_map: BTreeMap<String, Vec<(String, TargetType, &ArtifactDef)>> =
            BTreeMap::new();

        // Collect from NixOS configurations
        for (machine, artifacts) in &self.nixos_map {
            for (art_name, artifact) in artifacts {
                if artifact.shared {
                    shared_map.entry(art_name.clone()).or_default().push((
                        machine.clone(),
                        TargetType::Nixos,
                        artifact,
                    ));
                }
            }
        }

        // Collect from home-manager configurations
        for (user, artifacts) in &self.home_map {
            for (art_name, artifact) in artifacts {
                if artifact.shared {
                    shared_map.entry(art_name.clone()).or_default().push((
                        user.clone(),
                        TargetType::HomeManager,
                        artifact,
                    ));
                }
            }
        }

        // Build SharedArtifactInfo for each shared artifact
        let mut result: BTreeMap<String, SharedArtifactInfo> = BTreeMap::new();

        for (artifact_name, entries) in shared_map {
            // Group generators by path
            let mut generator_map: BTreeMap<String, Vec<GeneratorSource>> = BTreeMap::new();
            let mut nixos_targets: Vec<String> = Vec::new();
            let mut home_targets: Vec<String> = Vec::new();

            // Use the first definition for prompts, files, and backend
            let first_artifact = entries.first().map(|(_, _, a)| *a).unwrap();
            let backend_name = first_artifact.serialization.clone();
            let prompts = first_artifact.prompts.clone();
            let files = first_artifact.files.clone();

            for (target, target_type, artifact) in entries {
                // Add to generator map
                generator_map
                    .entry(artifact.generator.clone())
                    .or_default()
                    .push(GeneratorSource {
                        target: target.clone(),
                        target_type: target_type.clone(),
                    });

                // Add to target lists
                match target_type {
                    TargetType::Nixos => nixos_targets.push(target),
                    TargetType::HomeManager => home_targets.push(target),
                }
            }

            // Convert generator map to GeneratorInfo vec
            let generators: Vec<GeneratorInfo> = generator_map
                .into_iter()
                .map(|(path, sources)| GeneratorInfo { path, sources })
                .collect();

            result.insert(
                artifact_name.clone(),
                SharedArtifactInfo {
                    artifact_name,
                    generators,
                    nixos_targets,
                    home_targets,
                    backend_name,
                    prompts,
                    files,
                },
            );
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_temp_make_json(content: &str) -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let json_path = temp_dir.path().join("make.json");
        let mut file = fs::File::create(&json_path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        (temp_dir, json_path)
    }

    #[test]
    fn test_get_shared_artifacts_empty() {
        let content = r#"{"nixos": [], "home": []}"#;
        let (_temp_dir, json_path) = create_temp_make_json(content);
        let config = MakeConfiguration::read_make_config(&json_path).unwrap();

        let shared = config.get_shared_artifacts();
        assert!(shared.is_empty());
    }

    #[test]
    fn test_get_shared_artifacts_no_shared() {
        let content = r#"{
            "nixos": [{
                "machine": "machine-one",
                "artifacts": {
                    "my-secret": {
                        "name": "my-secret",
                        "shared": false,
                        "files": {},
                        "prompts": {},
                        "generator": "/nix/store/gen.sh",
                        "serialization": "test"
                    }
                },
                "config": {}
            }],
            "home": []
        }"#;
        let (_temp_dir, json_path) = create_temp_make_json(content);
        let config = MakeConfiguration::read_make_config(&json_path).unwrap();

        let shared = config.get_shared_artifacts();
        assert!(shared.is_empty());
    }

    #[test]
    fn test_get_shared_artifacts_single_machine() {
        let content = r#"{
            "nixos": [{
                "machine": "machine-one",
                "artifacts": {
                    "shared-secret": {
                        "name": "shared-secret",
                        "shared": true,
                        "files": {},
                        "prompts": {},
                        "generator": "/nix/store/gen.sh",
                        "serialization": "test"
                    }
                },
                "config": {}
            }],
            "home": []
        }"#;
        let (_temp_dir, json_path) = create_temp_make_json(content);
        let config = MakeConfiguration::read_make_config(&json_path).unwrap();

        let shared = config.get_shared_artifacts();
        assert_eq!(shared.len(), 1);

        let info = shared.get("shared-secret").unwrap();
        assert_eq!(info.artifact_name, "shared-secret");
        assert_eq!(info.nixos_targets, vec!["machine-one"]);
        assert!(info.home_targets.is_empty());
        assert_eq!(info.generators.len(), 1);
        assert_eq!(info.generators[0].path, "/nix/store/gen.sh");
        assert_eq!(info.backend_name, "test");
    }

    #[test]
    fn test_get_shared_artifacts_multiple_machines_same_generator() {
        let content = r#"{
            "nixos": [
                {
                    "machine": "machine-one",
                    "artifacts": {
                        "shared-secret": {
                            "name": "shared-secret",
                            "shared": true,
                            "files": {},
                            "prompts": {},
                            "generator": "/nix/store/gen.sh",
                            "serialization": "test"
                        }
                    },
                    "config": {}
                },
                {
                    "machine": "machine-two",
                    "artifacts": {
                        "shared-secret": {
                            "name": "shared-secret",
                            "shared": true,
                            "files": {},
                            "prompts": {},
                            "generator": "/nix/store/gen.sh",
                            "serialization": "test"
                        }
                    },
                    "config": {}
                }
            ],
            "home": []
        }"#;
        let (_temp_dir, json_path) = create_temp_make_json(content);
        let config = MakeConfiguration::read_make_config(&json_path).unwrap();

        let shared = config.get_shared_artifacts();
        assert_eq!(shared.len(), 1);

        let info = shared.get("shared-secret").unwrap();
        assert_eq!(info.nixos_targets.len(), 2);
        assert!(info.nixos_targets.contains(&"machine-one".to_string()));
        assert!(info.nixos_targets.contains(&"machine-two".to_string()));
        // Same generator, so only one GeneratorInfo
        assert_eq!(info.generators.len(), 1);
        // But it has sources from both machines
        assert_eq!(info.generators[0].sources.len(), 2);
    }

    #[test]
    fn test_get_shared_artifacts_multiple_machines_different_generators() {
        let content = r#"{
            "nixos": [
                {
                    "machine": "machine-one",
                    "artifacts": {
                        "shared-secret": {
                            "name": "shared-secret",
                            "shared": true,
                            "files": {},
                            "prompts": {},
                            "generator": "/nix/store/gen-a.sh",
                            "serialization": "test"
                        }
                    },
                    "config": {}
                },
                {
                    "machine": "machine-two",
                    "artifacts": {
                        "shared-secret": {
                            "name": "shared-secret",
                            "shared": true,
                            "files": {},
                            "prompts": {},
                            "generator": "/nix/store/gen-b.sh",
                            "serialization": "test"
                        }
                    },
                    "config": {}
                }
            ],
            "home": []
        }"#;
        let (_temp_dir, json_path) = create_temp_make_json(content);
        let config = MakeConfiguration::read_make_config(&json_path).unwrap();

        let shared = config.get_shared_artifacts();
        assert_eq!(shared.len(), 1);

        let info = shared.get("shared-secret").unwrap();
        // Different generators, so two GeneratorInfo entries
        assert_eq!(info.generators.len(), 2);
        let paths: Vec<&str> = info.generators.iter().map(|g| g.path.as_str()).collect();
        assert!(paths.contains(&"/nix/store/gen-a.sh"));
        assert!(paths.contains(&"/nix/store/gen-b.sh"));
    }

    #[test]
    fn test_get_shared_artifacts_mixed_nixos_and_home() {
        let content = r#"{
            "nixos": [{
                "machine": "server",
                "artifacts": {
                    "shared-secret": {
                        "name": "shared-secret",
                        "shared": true,
                        "files": {},
                        "prompts": {},
                        "generator": "/nix/store/gen.sh",
                        "serialization": "test"
                    }
                },
                "config": {}
            }],
            "home": [{
                "user": "alice@workstation",
                "artifacts": {
                    "shared-secret": {
                        "name": "shared-secret",
                        "shared": true,
                        "files": {},
                        "prompts": {},
                        "generator": "/nix/store/gen.sh",
                        "serialization": "test"
                    }
                },
                "config": {}
            }]
        }"#;
        let (_temp_dir, json_path) = create_temp_make_json(content);
        let config = MakeConfiguration::read_make_config(&json_path).unwrap();

        let shared = config.get_shared_artifacts();
        assert_eq!(shared.len(), 1);

        let info = shared.get("shared-secret").unwrap();
        assert_eq!(info.nixos_targets, vec!["server"]);
        assert_eq!(info.home_targets, vec!["alice@workstation"]);
        assert_eq!(info.generators.len(), 1);
        assert_eq!(info.generators[0].sources.len(), 2);

        // Check that sources have correct target types
        let nixos_sources: Vec<_> = info.generators[0]
            .sources
            .iter()
            .filter(|s| s.target_type == TargetType::Nixos)
            .collect();
        let home_sources: Vec<_> = info.generators[0]
            .sources
            .iter()
            .filter(|s| s.target_type == TargetType::HomeManager)
            .collect();
        assert_eq!(nixos_sources.len(), 1);
        assert_eq!(home_sources.len(), 1);
    }
}
