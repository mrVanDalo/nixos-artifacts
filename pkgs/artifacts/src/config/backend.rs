use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BackendSettings(pub HashMap<String, serde_json::Value>);

/// Explicit capability declarations for a backend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendCapabilities {
    /// Whether backend supports shared artifacts (multi-machine serialization).
    /// If not specified, inferred from presence of `shared_serialize` script.
    #[serde(default)]
    pub shared: Option<bool>,

    /// Whether backend actually serializes/persists secrets.
    /// False means passthrough mode (e.g., for testing or plaintext backends).
    /// Defaults to true if not specified.
    #[serde(default = "default_true")]
    pub serializes: bool,
}

impl Default for BackendCapabilities {
    fn default() -> Self {
        Self {
            shared: None,
            serializes: true,
        }
    }
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BackendEntry {
    /// Script to check if NixOS serialization is needed. Required if serializes=true.
    #[serde(default)]
    pub nixos_check_serialization: Option<String>,
    /// Script to serialize NixOS secrets. Required if serializes=true.
    #[serde(default)]
    pub nixos_serialize: Option<String>,
    /// Script to check if home-manager serialization is needed. Required if serializes=true.
    #[serde(default)]
    pub home_check_serialization: Option<String>,
    /// Script to serialize home-manager secrets. Required if serializes=true.
    #[serde(default)]
    pub home_serialize: Option<String>,
    /// Script to check if shared serialization is needed. Required if shared=true and serializes=true.
    #[serde(default)]
    pub shared_check_serialization: Option<String>,
    /// Script to serialize shared secrets. Required if shared=true and serializes=true.
    #[serde(default)]
    pub shared_serialize: Option<String>,
    #[serde(default)]
    pub settings: BackendSettings,
    #[serde(default)]
    pub capabilities: BackendCapabilities,
}

impl BackendEntry {
    /// Check if this backend supports shared artifacts.
    /// Uses explicit capability if declared, otherwise infers from shared_serialize presence.
    pub fn supports_shared(&self) -> bool {
        self.capabilities
            .shared
            .unwrap_or_else(|| self.shared_serialize.is_some())
    }
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

        // Get the directory containing this file for relative path resolution
        let base_dir = canonical.parent().unwrap_or(Path::new("."));

        // Resolve script paths relative to this file's directory
        let mut result: HashMap<String, BackendEntry> = HashMap::new();

        for (name, entry) in raw.backends {
            let serializes = entry.capabilities.serializes;

            // Validate: if serializes=true (default), require serialization scripts
            if serializes {
                // NixOS scripts required
                if entry.nixos_check_serialization.is_none() {
                    anyhow::bail!(
                        "backend '{}' requires 'nixos_check_serialization' script (serializes=true) in {}",
                        name,
                        toml_path.display()
                    );
                }
                if entry.nixos_serialize.is_none() {
                    anyhow::bail!(
                        "backend '{}' requires 'nixos_serialize' script (serializes=true) in {}",
                        name,
                        toml_path.display()
                    );
                }

                // Home-manager scripts required
                if entry.home_check_serialization.is_none() {
                    anyhow::bail!(
                        "backend '{}' requires 'home_check_serialization' script (serializes=true) in {}",
                        name,
                        toml_path.display()
                    );
                }
                if entry.home_serialize.is_none() {
                    anyhow::bail!(
                        "backend '{}' requires 'home_serialize' script (serializes=true) in {}",
                        name,
                        toml_path.display()
                    );
                }

                // If shared=true and serializes=true, require shared scripts
                if entry.capabilities.shared == Some(true) {
                    if entry.shared_check_serialization.is_none() {
                        anyhow::bail!(
                            "backend '{}' declares shared=true but missing 'shared_check_serialization' script in {}",
                            name,
                            toml_path.display()
                        );
                    }
                    if entry.shared_serialize.is_none() {
                        anyhow::bail!(
                            "backend '{}' declares shared=true but missing 'shared_serialize' script in {}",
                            name,
                            toml_path.display()
                        );
                    }
                }
            }

            let resolved_entry = BackendEntry {
                nixos_check_serialization: entry
                    .nixos_check_serialization
                    .map(|p| Self::resolve_script_path(base_dir, &p)),
                nixos_serialize: entry
                    .nixos_serialize
                    .map(|p| Self::resolve_script_path(base_dir, &p)),
                home_check_serialization: entry
                    .home_check_serialization
                    .map(|p| Self::resolve_script_path(base_dir, &p)),
                home_serialize: entry
                    .home_serialize
                    .map(|p| Self::resolve_script_path(base_dir, &p)),
                shared_check_serialization: entry
                    .shared_check_serialization
                    .map(|p| Self::resolve_script_path(base_dir, &p)),
                shared_serialize: entry
                    .shared_serialize
                    .map(|p| Self::resolve_script_path(base_dir, &p)),
                settings: entry.settings,
                capabilities: entry.capabilities,
            };
            result.insert(name, resolved_entry);
        }

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

    /// Resolve a script path relative to the given base directory.
    /// If the path is already absolute, return it as-is.
    /// Otherwise, join with base_dir and convert to string.
    fn resolve_script_path(base_dir: &Path, script_path: &str) -> String {
        let path = Path::new(script_path);
        if path.is_absolute() {
            script_path.to_string()
        } else {
            base_dir.join(path).to_string_lossy().to_string()
        }
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

    /// Validate that a backend supports shared serialization.
    /// Returns an error if the backend doesn't have shared scripts configured.
    pub fn validate_shared_serialize(&self, backend_name: &str) -> Result<()> {
        let backend = self.config.get(backend_name).with_context(|| {
            format!(
                "backend '{}' not found in {}",
                backend_name,
                self.backend_toml.display()
            )
        })?;

        if backend.shared_serialize.is_none() {
            anyhow::bail!(
                "backend '{}' does not support shared artifacts: \
                 missing 'shared_serialize' script in {}",
                backend_name,
                self.backend_toml.display()
            );
        }

        if backend.shared_check_serialization.is_none() {
            anyhow::bail!(
                "backend '{}' does not support shared artifacts: \
                 missing 'shared_check_serialization' script in {}",
                backend_name,
                self.backend_toml.display()
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_temp_backend_toml(content: &str) -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let toml_path = temp_dir.path().join("backend.toml");
        let mut file = fs::File::create(&toml_path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        (temp_dir, toml_path)
    }

    #[test]
    fn test_parse_backend_without_shared_scripts() {
        let content = r#"
[test]
nixos_check_serialization = "./nixos_check.sh"
nixos_serialize = "./nixos_serialize.sh"
home_check_serialization = "./home_check.sh"
home_serialize = "./home_serialize.sh"
"#;
        let (_temp_dir, toml_path) = create_temp_backend_toml(content);
        let config = BackendConfiguration::read_backend_config(&toml_path).unwrap();

        let backend = config.get_backend(&"test".to_string()).unwrap();
        assert!(backend.shared_serialize.is_none());
        assert!(backend.shared_check_serialization.is_none());
    }

    #[test]
    fn test_parse_backend_with_shared_scripts() {
        let content = r#"
[test]
nixos_check_serialization = "./nixos_check.sh"
nixos_serialize = "./nixos_serialize.sh"
home_check_serialization = "./home_check.sh"
home_serialize = "./home_serialize.sh"
shared_check_serialization = "./shared_check.sh"
shared_serialize = "./shared_serialize.sh"
"#;
        let (_temp_dir, toml_path) = create_temp_backend_toml(content);
        let config = BackendConfiguration::read_backend_config(&toml_path).unwrap();

        let backend = config.get_backend(&"test".to_string()).unwrap();
        assert!(backend.shared_serialize.is_some());
        assert!(backend.shared_check_serialization.is_some());
        assert!(backend
            .shared_serialize
            .unwrap()
            .ends_with("shared_serialize.sh"));
    }

    #[test]
    fn test_validate_shared_serialize_missing() {
        let content = r#"
[test]
nixos_check_serialization = "./nixos_check.sh"
nixos_serialize = "./nixos_serialize.sh"
home_check_serialization = "./home_check.sh"
home_serialize = "./home_serialize.sh"
"#;
        let (_temp_dir, toml_path) = create_temp_backend_toml(content);
        let config = BackendConfiguration::read_backend_config(&toml_path).unwrap();

        let validation_result = config.validate_shared_serialize("test");
        assert!(validation_result.is_err());
        let error_message = validation_result.unwrap_err().to_string();
        assert!(error_message.contains("does not support shared artifacts"));
        assert!(error_message.contains("missing 'shared_serialize'"));
    }

    #[test]
    fn test_validate_shared_serialize_present() {
        let content = r#"
[test]
nixos_check_serialization = "./nixos_check.sh"
nixos_serialize = "./nixos_serialize.sh"
home_check_serialization = "./home_check.sh"
home_serialize = "./home_serialize.sh"
shared_check_serialization = "./shared_check.sh"
shared_serialize = "./shared_serialize.sh"
"#;
        let (_temp_dir, toml_path) = create_temp_backend_toml(content);
        let config = BackendConfiguration::read_backend_config(&toml_path).unwrap();

        let validation_result = config.validate_shared_serialize("test");
        assert!(validation_result.is_ok());
    }

    #[test]
    fn test_validate_shared_serialize_backend_not_found() {
        let content = r#"
[test]
nixos_check_serialization = "./nixos_check.sh"
nixos_serialize = "./nixos_serialize.sh"
home_check_serialization = "./home_check.sh"
home_serialize = "./home_serialize.sh"
"#;
        let (_temp_dir, toml_path) = create_temp_backend_toml(content);
        let config = BackendConfiguration::read_backend_config(&toml_path).unwrap();

        let validation_result = config.validate_shared_serialize("nonexistent");
        assert!(validation_result.is_err());
        let error_message = validation_result.unwrap_err().to_string();
        assert!(error_message.contains("not found"));
    }

    #[test]
    fn test_parse_backend_with_explicit_capabilities() {
        let content = r#"
[test]
shared_serialize = "./shared_serialize.sh"
shared_check_serialization = "./shared_check.sh"

[test.capabilities]
shared = true
serializes = false
"#;
        let (_temp_dir, toml_path) = create_temp_backend_toml(content);
        let config = BackendConfiguration::read_backend_config(&toml_path).unwrap();

        let backend = config.get_backend(&"test".to_string()).unwrap();
        assert_eq!(backend.capabilities.shared, Some(true));
        assert!(!backend.capabilities.serializes);
        assert!(backend.supports_shared());
    }

    #[test]
    fn test_infer_shared_capability_from_script() {
        let content = r#"
[test]
nixos_check_serialization = "./nixos_check.sh"
nixos_serialize = "./nixos_serialize.sh"
home_check_serialization = "./home_check.sh"
home_serialize = "./home_serialize.sh"
shared_serialize = "./shared_serialize.sh"
shared_check_serialization = "./shared_check.sh"
"#;
        let (_temp_dir, toml_path) = create_temp_backend_toml(content);
        let config = BackendConfiguration::read_backend_config(&toml_path).unwrap();

        let backend = config.get_backend(&"test".to_string()).unwrap();
        // No explicit capability set
        assert_eq!(backend.capabilities.shared, None);
        // But supports_shared infers from shared_serialize presence
        assert!(backend.supports_shared());
    }

    #[test]
    fn test_shared_not_inferred_without_script() {
        let content = r#"
[test]
nixos_check_serialization = "./nixos_check.sh"
nixos_serialize = "./nixos_serialize.sh"
home_check_serialization = "./home_check.sh"
home_serialize = "./home_serialize.sh"
"#;
        let (_temp_dir, toml_path) = create_temp_backend_toml(content);
        let config = BackendConfiguration::read_backend_config(&toml_path).unwrap();

        let backend = config.get_backend(&"test".to_string()).unwrap();
        assert_eq!(backend.capabilities.shared, None);
        assert!(!backend.supports_shared());
    }

    #[test]
    fn test_serializes_defaults_to_true() {
        let content = r#"
[test]
nixos_check_serialization = "./nixos_check.sh"
nixos_serialize = "./nixos_serialize.sh"
home_check_serialization = "./home_check.sh"
home_serialize = "./home_serialize.sh"
"#;
        let (_temp_dir, toml_path) = create_temp_backend_toml(content);
        let config = BackendConfiguration::read_backend_config(&toml_path).unwrap();

        let backend = config.get_backend(&"test".to_string()).unwrap();
        assert!(backend.capabilities.serializes);
    }

    #[test]
    fn test_explicit_shared_overrides_script_inference() {
        // Even with shared_serialize script, explicit shared=false takes precedence
        let content = r#"
[test]
nixos_check_serialization = "./nixos_check.sh"
nixos_serialize = "./nixos_serialize.sh"
home_check_serialization = "./home_check.sh"
home_serialize = "./home_serialize.sh"
shared_serialize = "./shared_serialize.sh"
shared_check_serialization = "./shared_check.sh"

[test.capabilities]
shared = false
"#;
        let (_temp_dir, toml_path) = create_temp_backend_toml(content);
        let config = BackendConfiguration::read_backend_config(&toml_path).unwrap();

        let backend = config.get_backend(&"test".to_string()).unwrap();
        assert_eq!(backend.capabilities.shared, Some(false));
        assert!(!backend.supports_shared());
    }

    #[test]
    fn test_shared_true_requires_shared_scripts() {
        // If capabilities.shared=true is explicit, shared scripts must be defined
        let content = r#"
[test]
nixos_check_serialization = "./nixos_check.sh"
nixos_serialize = "./nixos_serialize.sh"
home_check_serialization = "./home_check.sh"
home_serialize = "./home_serialize.sh"

[test.capabilities]
shared = true
"#;
        let (_temp_dir, toml_path) = create_temp_backend_toml(content);
        let read_result = BackendConfiguration::read_backend_config(&toml_path);

        assert!(read_result.is_err());
        let error_message = read_result.unwrap_err().to_string();
        assert!(error_message.contains("declares shared=true but missing"));
    }

    #[test]
    fn test_shared_true_with_scripts_is_valid() {
        // If capabilities.shared=true and shared scripts exist, it's valid
        let content = r#"
[test]
nixos_check_serialization = "./nixos_check.sh"
nixos_serialize = "./nixos_serialize.sh"
home_check_serialization = "./home_check.sh"
home_serialize = "./home_serialize.sh"
shared_check_serialization = "./shared_check.sh"
shared_serialize = "./shared_serialize.sh"

[test.capabilities]
shared = true
"#;
        let (_temp_dir, toml_path) = create_temp_backend_toml(content);
        let config = BackendConfiguration::read_backend_config(&toml_path).unwrap();

        let backend = config.get_backend(&"test".to_string()).unwrap();
        assert_eq!(backend.capabilities.shared, Some(true));
        assert!(backend.supports_shared());
    }

    #[test]
    fn test_serializes_false_scripts_optional() {
        // If serializes=false, no scripts are required
        let content = r#"
[test]

[test.capabilities]
serializes = false
"#;
        let (_temp_dir, toml_path) = create_temp_backend_toml(content);
        let config = BackendConfiguration::read_backend_config(&toml_path).unwrap();

        let backend = config.get_backend(&"test".to_string()).unwrap();
        assert!(!backend.capabilities.serializes);
        assert!(backend.nixos_check_serialization.is_none());
        assert!(backend.nixos_serialize.is_none());
        assert!(backend.home_check_serialization.is_none());
        assert!(backend.home_serialize.is_none());
    }

    #[test]
    fn test_serializes_false_shared_true_no_script_required() {
        // If serializes=false and shared=true, shared scripts are not required
        let content = r#"
[test]

[test.capabilities]
serializes = false
shared = true
"#;
        let (_temp_dir, toml_path) = create_temp_backend_toml(content);
        let config = BackendConfiguration::read_backend_config(&toml_path).unwrap();

        let backend = config.get_backend(&"test".to_string()).unwrap();
        assert!(!backend.capabilities.serializes);
        assert_eq!(backend.capabilities.shared, Some(true));
        assert!(backend.shared_serialize.is_none());
        assert!(backend.shared_check_serialization.is_none());
    }

    #[test]
    fn test_serializes_true_requires_scripts() {
        // If serializes=true (default), scripts are required
        let content = r#"
[test]
"#;
        let (_temp_dir, toml_path) = create_temp_backend_toml(content);
        let read_result = BackendConfiguration::read_backend_config(&toml_path);

        assert!(read_result.is_err());
        let error_message = read_result.unwrap_err().to_string();
        assert!(error_message.contains("requires 'nixos_check_serialization' script"));
    }
}
