//! Artifact configuration from Nix flake evaluation.
//!
//! This module handles the extraction and parsing of artifact definitions
//! from `flake.nix` files. It supports both NixOS configurations (machines)
//! and home-manager configurations (users).
//!
//! ## Configuration Flow
//!
//! 1. Nix expression is built via [`nix::build_make_expr`](super::nix)
//! 2. `nix eval` produces JSON output containing artifact definitions
//! 3. JSON is parsed into [`MakeConfiguration`] with typed structures
//! 4. TUI uses the configuration to drive artifact generation
//!
//! ## JSON Structure
//!
//! The expected JSON format from Nix evaluation:
//!
//! ```json
//! {
//!   "nixos": [{
//!     "machine": "machine-one",
//!     "artifacts": {
//!       "ssh-key": {
//!         "name": "ssh-key",
//!         "shared": false,
//!         "files": {
//!           "id_ed25519": {
//!             "name": "id_ed25519",
//!             "path": "/run/secrets/id_ed25519",
//!             "owner": "root",
//!             "group": "root"
//!           }
//!         },
//!         "prompts": {
//!           "passphrase": {
//!             "name": "passphrase",
//!             "description": "SSH key passphrase"
//!           }
//!         },
//!         "generator": "/nix/store/.../generate.sh",
//!         "serialization": "agenix"
//!       }
//!     },
//!     "config": {
//!       "agenix": { "publicKey": "ssh-ed25519 ..." }
//!     }
//!   }],
//!   "home": []
//! }
//! ```
//!
//! ## Artifact Types
//!
//! - **Per-machine artifacts**: Defined in `nixos` array, scoped to specific machines
//! - **Per-user artifacts**: Defined in `home` array, scoped to home-manager users
//! - **Shared artifacts**: Marked with `"shared": true`, shared across multiple targets

#[cfg(feature = "logging")]
use crate::log_debug;
use anyhow::Context;
use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize};
#[cfg(feature = "logging")]
use serde_json::to_string_pretty;
use serde_json::{Value, from_str as json_from_str};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// File definition within an artifact.
///
/// Specifies the properties of a single file that will be generated
/// and where it should be deployed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDef {
    /// Name identifier for this file within the artifact
    pub name: String,
    /// Target path where the file will be deployed
    pub path: Option<String>,
    /// File owner (NixOS only, system-level permissions)
    pub owner: Option<String>,
    /// File group (NixOS only, system-level permissions)
    pub group: Option<String>,
}

/// Prompt definition for user input during generation.
///
/// Prompts are collected from the user before running the generator
/// and passed to the generator script via environment variables.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptDef {
    /// Name identifier for this prompt (used as env var name)
    pub name: String,
    /// Human-readable description shown in the TUI
    pub description: Option<String>,
}

/// Complete artifact definition extracted from Nix configuration.
///
/// An artifact represents a logical secret bundle that produces one or more
/// deployable files. It includes:
/// - Files to be generated and their deployment properties
/// - Prompts for user input
/// - Generator script that produces the files
/// - Backend reference for serialization
#[derive(Debug, Clone, Serialize)]
pub struct ArtifactDef {
    /// Artifact name identifier
    pub name: String,
    /// Optional description for display in the TUI
    pub description: Option<String>,
    /// Whether this artifact is shared across multiple machines/users
    pub shared: bool,
    /// Files to be created, keyed by file name
    pub files: BTreeMap<String, FileDef>,
    /// Prompts to collect from user, keyed by prompt name
    pub prompts: BTreeMap<String, PromptDef>,
    /// Path to the generator script that produces files
    pub generator: String,
    /// Backend name for serialization (references backend.toml)
    pub serialization: String,
}

impl<'de> Deserialize<'de> for ArtifactDef {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct ArtifactDefHelper {
            name: Option<String>,
            description: Option<String>,
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
            description: helper.description,
            shared,
            files: helper.files,
            prompts: helper.prompts,
            generator,
            serialization,
        })
    }
}

/// Type of target that defines an artifact.
///
/// Artifacts can be defined in either NixOS machine configurations
/// or home-manager user configurations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum TargetType {
    /// NixOS machine configuration (system-level artifacts)
    Nixos,
    /// Home-manager user configuration (user-level artifacts)
    HomeManager,
}

/// Source of a generator - which target defines it.
///
/// Tracks which target (machine or user) provides a specific
/// generator script for shared artifact generation.
#[derive(Debug, Clone, Serialize)]
pub struct GeneratorSource {
    /// Machine name or user identifier (e.g., "machine-one" or "alice@workstation")
    pub target: String,
    /// Type of target (NixOS or HomeManager)
    pub target_type: TargetType,
}

/// A unique generator script with its sources.
///
/// For shared artifacts, multiple targets may use the same generator
/// script. This struct tracks the script path and all targets that
/// reference it.
#[derive(Debug, Clone, Serialize)]
pub struct GeneratorInfo {
    /// Path to the generator script (Nix store path)
    pub path: String,
    /// Targets that use this generator (for shared artifacts)
    pub sources: Vec<GeneratorSource>,
}

/// Aggregated information about a shared artifact across all targets.
///
/// Shared artifacts are marked with `shared: true` and can be used
/// across multiple NixOS machines and home-manager users. This
/// struct aggregates information from all targets for display and
/// generation purposes.
///
/// ## Validation
///
/// Shared artifacts must have identical file definitions across all
/// targets. If files mismatch, the `error` field contains a message.
#[derive(Debug, Clone, Serialize)]
pub struct SharedArtifactInfo {
    /// Artifact name identifier
    pub artifact_name: String,
    /// Artifact description (from first definition found)
    pub description: Option<String>,
    /// Unique generators with their sources (different targets may use different generators)
    pub generators: Vec<GeneratorInfo>,
    /// NixOS machine names that use this artifact
    pub nixos_targets: Vec<String>,
    /// Home-manager user identifiers that use this artifact
    pub home_targets: Vec<String>,
    /// Backend name for serialization (references backend.toml)
    pub backend_name: String,
    /// Prompts from first definition (shared artifacts must have identical prompts)
    pub prompts: BTreeMap<String, PromptDef>,
    /// Files from first definition (shared artifacts must have identical files)
    pub files: BTreeMap<String, FileDef>,
    /// Validation error if file definitions mismatch across targets
    pub error: Option<String>,
}

/// Central configuration structure extracted from Nix flake.
///
/// This is the primary data structure that holds all artifact definitions
/// extracted from `nix eval` of the flake.nix file. It contains separate
/// maps for NixOS machines and home-manager users.
///
/// ## Field Structure
///
/// - `nixos_map`: machine-name → (artifact-name → artifact)
/// - `home_map`: user-name → (artifact-name → artifact)
/// - `nixos_config`: machine-name → (backend-name → config-map)
/// - `home_config`: user-name → (backend-name → config-map)
/// - `make_base`: Directory containing the make.json file
/// - `make_json`: Path to the make.json file
#[derive(Debug, Clone)]
pub struct MakeConfiguration {
    /// NixOS machines: machine-name → (artifact-name → artifact)
    pub nixos_map: BTreeMap<String, BTreeMap<String, ArtifactDef>>,
    /// Home-manager users: user-name → (artifact-name → artifact)
    pub home_map: BTreeMap<String, BTreeMap<String, ArtifactDef>>,
    /// Per-machine backend configs: machine-name → (backend-name → config-map)
    pub nixos_config: BTreeMap<String, BTreeMap<String, BTreeMap<String, Value>>>,
    /// Per-user backend configs: user-name → (backend-name → config-map)
    pub home_config: BTreeMap<String, BTreeMap<String, BTreeMap<String, Value>>>,
    /// Directory containing the make.json file
    pub make_base: PathBuf,
    /// Path to the make.json file
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
    pub fn parse_make_config(make_text: &str, make_json: &Path) -> anyhow::Result<MakeConfiguration> {
        #[cfg(feature = "logging")]
        {
            let pretty = match json_from_str::<Value>(make_text) {
                Ok(v) => to_string_pretty(&v).unwrap_or_else(|_| make_text.to_string()),
                Err(_) => make_text.to_string(),
            };
            log_debug!("make config (pretty):\n{}", pretty);
        }

        let root: MakeRoot = json_from_str(make_text)
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

    pub fn read_make_config(make_json: &Path) -> anyhow::Result<MakeConfiguration> {
        let make_text = fs::read_to_string(make_json)
            .with_context(|| format!("reading make config {}", make_json.display()))?;
        Self::parse_make_config(&make_text, make_json)
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

            for (target, target_type, artifact) in &entries {
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
                    TargetType::Nixos => nixos_targets.push(target.clone()),
                    TargetType::HomeManager => home_targets.push(target.clone()),
                }
            }

            // Convert generator map to GeneratorInfo vec
            let generators: Vec<GeneratorInfo> = generator_map
                .into_iter()
                .map(|(path, sources)| GeneratorInfo { path, sources })
                .collect();

            // Validate file definitions match across all targets
            let error = validate_shared_files(&entries);

            result.insert(
                artifact_name.clone(),
                SharedArtifactInfo {
                    artifact_name,
                    description: first_artifact.description.clone(),
                    generators,
                    nixos_targets,
                    home_targets,
                    backend_name,
                    prompts,
                    files,
                    error,
                },
            );
        }

        result
    }
}

/// Validates that file definitions are identical across all targets for a shared artifact.
/// Returns Some(error_message) if files don't match, None otherwise.
fn validate_shared_files(entries: &[(String, TargetType, &ArtifactDef)]) -> Option<String> {
    if entries.len() < 2 {
        return None; // Single target, no comparison needed
    }

    // Get file names from first entry
    let first_artifact = entries.first().map(|(_, _, a)| *a).unwrap();
    let first_files: std::collections::BTreeSet<String> =
        first_artifact.files.keys().cloned().collect();

    // Compare with all other entries
    let mut mismatches: Vec<String> = Vec::new();

    for (target, _, artifact) in entries.iter().skip(1) {
        let other_files: std::collections::BTreeSet<String> =
            artifact.files.keys().cloned().collect();

        if first_files != other_files {
            let missing_in_other: Vec<_> = first_files
                .difference(&other_files)
                .map(|s| s.as_str())
                .collect();
            let extra_in_other: Vec<_> = other_files
                .difference(&first_files)
                .map(|s| s.as_str())
                .collect();

            let mut details = String::new();
            if !missing_in_other.is_empty() {
                details.push_str(&format!("missing: [{}]", missing_in_other.join(", ")));
            }
            if !extra_in_other.is_empty() {
                if !details.is_empty() {
                    details.push_str(", ");
                }
                details.push_str(&format!("extra: [{}]", extra_in_other.join(", ")));
            }

            mismatches.push(format!("{}: {}", target, details));
        }
    }

    if mismatches.is_empty() {
        None
    } else {
        Some(format!(
            "File definition mismatch: {} defines [{}], {}. Shared artifacts must have identical file definitions.",
            entries.first().map(|(t, _, _)| t.as_str()).unwrap(),
            first_files.into_iter().collect::<Vec<_>>().join(", "),
            mismatches.join("; ")
        ))
    }
}


