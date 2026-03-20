//! Backend configuration parsing from TOML files.
//!
//! This module handles parsing of `backend.toml` files that define serialization
//! backends for the artifacts CLI. Backends specify how artifacts are stored,
//! retrieved, and checked for serialization status.
//!
//! ## TOML Structure
//!
//! A backend.toml file defines one or more backends with per-target configuration:
//!
//! ```toml
//! [agenix.nixos]
//! enabled = true                    # Optional, defaults to true if scripts set
//! check = "./agenix_nixos_check.sh"
//! serialize = "./agenix_nixos_serialize.sh"
//!
//! [agenix.home]
//! enabled = true
//! check = "./agenix_home_check.sh"
//! serialize = "./agenix_home_serialize.sh"
//!
//! [agenix.shared]
//! enabled = true
//! check = "./agenix_shared_check.sh"
//! serialize = "./agenix_shared_serialize.sh"
//!
//! [agenix.settings]
//! key = "value"
//! ```
//!
//! ## Validation Rules
//!
//! ### check and serialize Pairing
//!
//! | `check` | `serialize` | Result |
//! | ------- | ----------- | ------ |
//! | absent  | absent      | Valid: `serializes = false` (passthrough mode) |
//! | present | present     | Valid: `serializes = true` |
//! | present | absent      | **ERROR**: "check requires serialize" |
//! | absent  | present     | **ERROR**: "serialize requires check" |
//!
//! ### enabled Inference Rules
//!
//! | Condition | Inferred `enabled` | Inferred `serializes` |
//! | --------- | ------------------ | ---------------------- |
//! | Section absent | `false` | N/A |
//! | Section present, no scripts, no `enabled` | `false` (implicit) | `false` |
//! | Section present, no scripts, `enabled = true` | `true` (explicit) | `false` |
//! | Section present, both scripts, no `enabled` | `true` (default) | `true` |
//! | Section present, both scripts, `enabled = true` | `true` (explicit) | `true` |
//! | Section present, both scripts, `enabled = false` | `false` (explicit) | `true` |
//! | Section present, one script only | **ERROR** | — |
//!
//! ### supports_shared Inference
//!
//! - `true` if `[backend.shared]` section exists AND `enabled = true` (explicit or inferred)
//! - `false` otherwise
//!
//! ## Include Directive
//!
//! Backend configuration can be split across multiple files using the `include` directive:
//!
//! ```toml
//! include = ["./backends/agenix.toml", "./backends/sops.toml"]
//!
//! [test]
//! nixos_check_serialization = "./test_check.sh"
//! nixos_serialize = "./test_serialize.sh"
//! ```
//!
//! Paths in `include` are resolved relative to the file containing the directive.
//! Circular includes are detected and rejected.
//!
//! ## Script Paths
//!
//! Script paths can be absolute or relative to the backend.toml file:
//! - `"./scripts/check.sh"` - relative to the TOML file
//! - `"/usr/local/bin/check.sh"` - absolute path

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Backend-specific settings as a flexible key-value map.
///
/// This type allows backends to declare arbitrary configuration settings
/// that are passed through to serialization scripts via environment variables.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BackendSettings(pub HashMap<String, serde_json::Value>);

/// Target type for configuration sections (nixos, home, shared).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TargetType {
    NixOS,
    Home,
    Shared,
}

impl std::fmt::Display for TargetType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TargetType::NixOS => write!(f, "nixos"),
            TargetType::Home => write!(f, "home"),
            TargetType::Shared => write!(f, "shared"),
        }
    }
}

/// Configuration for a single target (nixos, home, or shared).
///
/// Each target can have:
/// - `enabled`: Optional boolean to explicitly enable/disable the target
/// - `check`: Script to check if serialization is needed
/// - `serialize`: Script to serialize artifacts
///
/// The `check` and `serialize` scripts must be provided together or both omitted.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TargetConfig {
    /// Whether this target is enabled. None = infer from scripts presence.
    #[serde(default)]
    pub enabled: Option<bool>,
    /// Script to check if serialization is needed.
    #[serde(default)]
    pub check: Option<String>,
    /// Script to serialize artifacts.
    #[serde(default)]
    pub serialize: Option<String>,
}

impl TargetConfig {
    /// Returns true if this target is enabled.
    ///
    /// Enabled status is determined by:
    /// 1. Explicit `enabled` field if set
    /// 2. Inferred from presence of both check and serialize scripts
    /// 3. Default: false if section exists with no scripts
    pub fn is_enabled(&self) -> bool {
        match self.enabled {
            Some(explicit) => explicit,
            None => self.check.is_some() && self.serialize.is_some(),
        }
    }

    /// Returns true if this target has both check and serialize scripts (serializes).
    ///
    /// A target serializes if both `check` and `serialize` scripts are present.
    /// If only one is present, it's a configuration error.
    pub fn serializes(&self) -> bool {
        self.check.is_some() && self.serialize.is_some()
    }

    /// Validates that check and serialize are properly paired.
    ///
    /// Returns an error if:
    /// - `check` is present without `serialize`
    /// - `serialize` is present without `check`
    pub fn validate(&self, target_type: TargetType, backend_name: &str) -> Result<()> {
        match (&self.check, &self.serialize) {
            (Some(_), None) => {
                bail!(
                    "backend '{}.{}': 'check' requires 'serialize' to be defined",
                    backend_name,
                    target_type
                );
            }
            (None, Some(_)) => {
                bail!(
                    "backend '{}.{}': 'serialize' requires 'check' to be defined",
                    backend_name,
                    target_type
                );
            }
            _ => Ok(()),
        }
    }
}

/// Complete backend definition with per-target configuration.
///
/// A backend defines the complete lifecycle for artifact serialization:
/// - Check: Determine if serialization is needed
/// - Serialize: Store generated artifacts
/// - Shared: For multi-machine artifacts
///
/// Each target (nixos, home, shared) is independently configurable.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BackendEntry {
    /// NixOS target configuration.
    #[serde(default)]
    pub nixos: Option<TargetConfig>,
    /// Home Manager target configuration.
    #[serde(default)]
    pub home: Option<TargetConfig>,
    /// Shared artifacts configuration (for multi-machine artifacts).
    #[serde(default)]
    pub shared: Option<TargetConfig>,
    /// Backend-specific settings from `[backend_name.settings]` table.
    #[serde(default)]
    pub settings: BackendSettings,
}

impl BackendEntry {
    /// Check if this backend supports shared artifacts.
    ///
    /// Returns true if:
    /// - The `[backend.shared]` section exists
    /// - AND the shared target is enabled
    pub fn supports_shared(&self) -> bool {
        self.shared
            .as_ref()
            .is_some_and(|s| s.is_enabled() && s.serializes())
    }

    /// Check if a specific target is enabled.
    pub fn target_enabled(&self, target: TargetType) -> bool {
        match target {
            TargetType::NixOS => self.nixos.as_ref().is_some_and(|t| t.is_enabled()),
            TargetType::Home => self.home.as_ref().is_some_and(|t| t.is_enabled()),
            TargetType::Shared => self.shared.as_ref().is_some_and(|t| t.is_enabled()),
        }
    }

    /// Get check script for a target type.
    pub fn check_script(&self, target: TargetType) -> Option<&String> {
        match target {
            TargetType::NixOS => self.nixos.as_ref()?.check.as_ref(),
            TargetType::Home => self.home.as_ref()?.check.as_ref(),
            TargetType::Shared => self.shared.as_ref()?.check.as_ref(),
        }
    }

    /// Get serialize script for a target type.
    pub fn serialize_script(&self, target: TargetType) -> Option<&String> {
        match target {
            TargetType::NixOS => self.nixos.as_ref()?.serialize.as_ref(),
            TargetType::Home => self.home.as_ref()?.serialize.as_ref(),
            TargetType::Shared => self.shared.as_ref()?.serialize.as_ref(),
        }
    }

    /// Validate all target configurations.
    pub fn validate(&self, backend_name: &str) -> Result<()> {
        if let Some(ref nixos) = self.nixos {
            nixos.validate(TargetType::NixOS, backend_name)?;
        }
        if let Some(ref home) = self.home {
            home.validate(TargetType::Home, backend_name)?;
        }
        if let Some(ref shared) = self.shared {
            shared.validate(TargetType::Shared, backend_name)?;
        }
        Ok(())
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

/// Container combining backend configuration and its base path.
///
/// This struct holds the parsed backend configuration along with metadata
/// about where it was loaded from, which is needed for resolving relative
/// script paths and error reporting.
///
/// ## Field Relationships
///
/// - `config`: Map of backend name to [`BackendEntry`]
/// - `base_path`: Directory containing the primary backend.toml file
/// - `backend_toml`: Full path to the primary backend.toml file
#[derive(Debug, Clone)]
pub struct BackendConfiguration {
    pub config: HashMap<String, BackendEntry>,
    pub base_path: PathBuf,
    pub backend_toml: PathBuf,
}

impl BackendConfiguration {
    /// Load and parse a backend.toml file with support for includes.
    ///
    /// This function reads the primary backend.toml file and recursively
    /// processes any `include` directives to build a complete configuration.
    ///
    /// ## Arguments
    ///
    /// - `backend_toml`: Path to the primary backend.toml file
    ///
    /// ## Returns
    ///
    /// A [`BackendConfiguration`] containing the parsed backends and path metadata.
    ///
    /// ## Errors
    ///
    /// Returns an error if:
    /// - The file cannot be read
    /// - The TOML is malformed
    /// - A circular include is detected
    /// - Required scripts are missing for a backend
    /// - A duplicate backend name is found across included files
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// use std::path::Path;
    ///
    /// let config = BackendConfiguration::read_backend_config(
    ///     Path::new("./backend.toml")
    /// )?;
    /// ```
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

    /// Recursively load backend configuration with circular include detection.
    ///
    /// This internal method handles the recursive loading of backend.toml files
    /// and their included files. It validates script requirements and resolves
    /// relative paths to absolute paths.
    ///
    /// ## Arguments
    ///
    /// - `toml_path`: Path to the TOML file to load
    /// - `visited`: Set of already-visited canonical paths (for circular detection)
    ///
    /// ## Returns
    ///
    /// A map of backend names to their [`BackendEntry`] definitions.
    fn load_with_includes(
        toml_path: &Path,
        visited: &mut HashSet<PathBuf>,
    ) -> Result<HashMap<String, BackendEntry>> {
        let canonical = toml_path
            .canonicalize()
            .with_context(|| format!("resolving path {}", toml_path.display()))?;

        if !visited.insert(canonical.clone()) {
            bail!("circular include detected: {}", toml_path.display());
        }

        let text = fs::read_to_string(&canonical)
            .with_context(|| format!("reading backend config {}", toml_path.display()))?;

        let raw: BackendFileRaw = toml::from_str(&text)
            .with_context(|| format!("parsing backend config {}", toml_path.display()))?;

        let base_dir = canonical.parent().unwrap_or(Path::new("."));

        let mut result: HashMap<String, BackendEntry> = HashMap::new();

        for (name, entry) in raw.backends {
            entry.validate(&name)?;

            let resolved_entry = BackendEntry {
                nixos: entry
                    .nixos
                    .map(|t| Self::resolve_target_config(base_dir, t)),
                home: entry.home.map(|t| Self::resolve_target_config(base_dir, t)),
                shared: entry
                    .shared
                    .map(|t| Self::resolve_target_config(base_dir, t)),
                settings: entry.settings,
            };
            result.insert(name, resolved_entry);
        }

        for include_path in raw.include {
            let resolved_path = base_dir.join(&include_path);
            let included = Self::load_with_includes(&resolved_path, visited)?;

            for (key, value) in included {
                if result.contains_key(&key) {
                    bail!(
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

    fn resolve_target_config(base_dir: &Path, target: TargetConfig) -> TargetConfig {
        TargetConfig {
            enabled: target.enabled,
            check: target
                .check
                .map(|p| Self::resolve_script_path(base_dir, &p)),
            serialize: target
                .serialize
                .map(|p| Self::resolve_script_path(base_dir, &p)),
        }
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

        if !backend.supports_shared() {
            bail!(
                "backend '{}' does not support shared artifacts: \
                 missing or disabled shared target in {}",
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
    fn test_new_format_full_serializing_backend() {
        let content = r#"
[test.nixos]
check = "./check.sh"
serialize = "./serialize.sh"

[test.home]
check = "./check.sh"
serialize = "./serialize.sh"
"#;
        let (_temp_dir, toml_path) = create_temp_backend_toml(content);
        let config = BackendConfiguration::read_backend_config(&toml_path).unwrap();

        let backend = config.get_backend(&"test".to_string()).unwrap();
        assert!(backend.nixos.is_some());
        assert!(backend.home.is_some());
        assert!(backend.shared.is_none());

        let nixos = backend.nixos.unwrap();
        assert!(nixos.check.is_some());
        assert!(nixos.serialize.is_some());
        assert!(nixos.is_enabled());
        assert!(nixos.serializes());
    }

    #[test]
    fn test_new_format_backend_with_shared() {
        let content = r#"
[test.nixos]
check = "./check.sh"
serialize = "./serialize.sh"

[test.home]
check = "./check.sh"
serialize = "./serialize.sh"

[test.shared]
check = "./shared_check.sh"
serialize = "./shared_serialize.sh"
"#;
        let (_temp_dir, toml_path) = create_temp_backend_toml(content);
        let config = BackendConfiguration::read_backend_config(&toml_path).unwrap();

        let backend = config.get_backend(&"test".to_string()).unwrap();
        assert!(backend.supports_shared());

        let shared = backend.shared.unwrap();
        assert!(shared.is_enabled());
        assert!(shared.serializes());
    }

    #[test]
    fn test_new_format_enabled_inference_with_scripts() {
        let content = r#"
[test.nixos]
check = "./check.sh"
serialize = "./serialize.sh"
"#;
        let (_temp_dir, toml_path) = create_temp_backend_toml(content);
        let config = BackendConfiguration::read_backend_config(&toml_path).unwrap();

        let backend = config.get_backend(&"test".to_string()).unwrap();
        assert!(backend.target_enabled(TargetType::NixOS));
        assert!(!backend.target_enabled(TargetType::Home));
        assert!(!backend.target_enabled(TargetType::Shared));
    }

    #[test]
    fn test_new_format_explicit_enabled_false_with_scripts() {
        let content = r#"
[test.nixos]
enabled = false
check = "./check.sh"
serialize = "./serialize.sh"
"#;
        let (_temp_dir, toml_path) = create_temp_backend_toml(content);
        let config = BackendConfiguration::read_backend_config(&toml_path).unwrap();

        let backend = config.get_backend(&"test".to_string()).unwrap();
        assert!(!backend.target_enabled(TargetType::NixOS));

        let nixos = backend.nixos.unwrap();
        assert!(!nixos.is_enabled());
        assert!(nixos.serializes());
    }

    #[test]
    fn test_new_format_passthrough_mode_no_scripts() {
        let content = r#"
[test.nixos]
enabled = true

[test.home]
enabled = true
"#;
        let (_temp_dir, toml_path) = create_temp_backend_toml(content);
        let config = BackendConfiguration::read_backend_config(&toml_path).unwrap();

        let backend = config.get_backend(&"test".to_string()).unwrap();
        assert!(backend.target_enabled(TargetType::NixOS));
        assert!(backend.target_enabled(TargetType::Home));

        let nixos = backend.nixos.unwrap();
        assert!(nixos.is_enabled());
        assert!(!nixos.serializes());
    }

    #[test]
    fn test_new_format_check_without_serialize_error() {
        let content = r#"
[test.nixos]
check = "./check.sh"
"#;
        let (_temp_dir, toml_path) = create_temp_backend_toml(content);
        let result = BackendConfiguration::read_backend_config(&toml_path);

        assert!(result.is_err());
        let error_message = result.unwrap_err().to_string();
        assert!(error_message.contains("'check' requires 'serialize'"));
    }

    #[test]
    fn test_new_format_serialize_without_check_error() {
        let content = r#"
[test.nixos]
serialize = "./serialize.sh"
"#;
        let (_temp_dir, toml_path) = create_temp_backend_toml(content);
        let result = BackendConfiguration::read_backend_config(&toml_path);

        assert!(result.is_err());
        let error_message = result.unwrap_err().to_string();
        assert!(error_message.contains("'serialize' requires 'check'"));
    }

    #[test]
    fn test_new_format_inferred_enabled_from_scripts() {
        let content = r#"
[test.nixos]
check = "./check.sh"
serialize = "./serialize.sh"

[test.home]
enabled = true
"#;
        let (_temp_dir, toml_path) = create_temp_backend_toml(content);
        let config = BackendConfiguration::read_backend_config(&toml_path).unwrap();

        let backend = config.get_backend(&"test".to_string()).unwrap();
        assert!(backend.target_enabled(TargetType::NixOS));
        assert!(backend.target_enabled(TargetType::Home));

        let nixos = backend.nixos.as_ref().unwrap();
        assert_eq!(nixos.enabled, None);
        assert!(nixos.is_enabled());

        let home = backend.home.as_ref().unwrap();
        assert_eq!(home.enabled, Some(true));
        assert!(home.is_enabled());
        assert!(!home.serializes());
    }

    #[test]
    fn test_new_format_disabled_shared_target() {
        let content = r#"
[test.nixos]
check = "./check.sh"
serialize = "./serialize.sh"

[test.shared]
enabled = false
check = "./shared_check.sh"
serialize = "./shared_serialize.sh"
"#;
        let (_temp_dir, toml_path) = create_temp_backend_toml(content);
        let config = BackendConfiguration::read_backend_config(&toml_path).unwrap();

        let backend = config.get_backend(&"test".to_string()).unwrap();
        assert!(!backend.supports_shared());
        assert!(!backend.target_enabled(TargetType::Shared));
    }

    #[test]
    fn test_new_format_validate_shared_serialize_missing() {
        let content = r#"
[test.nixos]
check = "./check.sh"
serialize = "./serialize.sh"
"#;
        let (_temp_dir, toml_path) = create_temp_backend_toml(content);
        let config = BackendConfiguration::read_backend_config(&toml_path).unwrap();

        let result = config.validate_shared_serialize("test");
        assert!(result.is_err());
        let error_message = result.unwrap_err().to_string();
        assert!(error_message.contains("does not support shared artifacts"));
    }

    #[test]
    fn test_new_format_validate_shared_serialize_present() {
        let content = r#"
[test.nixos]
check = "./check.sh"
serialize = "./serialize.sh"

[test.shared]
check = "./shared_check.sh"
serialize = "./shared_serialize.sh"
"#;
        let (_temp_dir, toml_path) = create_temp_backend_toml(content);
        let config = BackendConfiguration::read_backend_config(&toml_path).unwrap();

        let result = config.validate_shared_serialize("test");
        assert!(result.is_ok());
    }

    #[test]
    fn test_new_format_partial_backend_nixos_only() {
        let content = r#"
[test.nixos]
check = "./check.sh"
serialize = "./serialize.sh"
"#;
        let (_temp_dir, toml_path) = create_temp_backend_toml(content);
        let config = BackendConfiguration::read_backend_config(&toml_path).unwrap();

        let backend = config.get_backend(&"test".to_string()).unwrap();
        assert!(backend.nixos.is_some());
        assert!(backend.home.is_none());
        assert!(backend.shared.is_none());
    }

    #[test]
    fn test_new_format_partial_backend_home_only() {
        let content = r#"
[test.home]
check = "./check.sh"
serialize = "./serialize.sh"
"#;
        let (_temp_dir, toml_path) = create_temp_backend_toml(content);
        let config = BackendConfiguration::read_backend_config(&toml_path).unwrap();

        let backend = config.get_backend(&"test".to_string()).unwrap();
        assert!(backend.nixos.is_none());
        assert!(backend.home.is_some());
        assert!(backend.shared.is_none());
    }

    #[test]
    fn test_new_format_backend_with_settings() {
        let content = r#"
[test.nixos]
check = "./check.sh"
serialize = "./serialize.sh"

[test.settings]
key = "value"
another = 123
"#;
        let (_temp_dir, toml_path) = create_temp_backend_toml(content);
        let config = BackendConfiguration::read_backend_config(&toml_path).unwrap();

        let backend = config.get_backend(&"test".to_string()).unwrap();
        assert!(backend.settings.0.contains_key("key"));
        assert!(backend.settings.0.contains_key("another"));
    }

    #[test]
    fn test_new_format_empty_backend_section() {
        let content = r#"
[test]
"#;
        let (_temp_dir, toml_path) = create_temp_backend_toml(content);
        let config = BackendConfiguration::read_backend_config(&toml_path).unwrap();

        let backend = config.get_backend(&"test".to_string()).unwrap();
        assert!(backend.nixos.is_none());
        assert!(backend.home.is_none());
        assert!(backend.shared.is_none());
    }

    #[test]
    fn test_new_format_multiple_backends() {
        let content = r#"
[backend1.nixos]
check = "./check1.sh"
serialize = "./serialize1.sh"

[backend2.nixos]
check = "./check2.sh"
serialize = "./serialize2.sh"

[backend2.home]
check = "./check2.sh"
serialize = "./serialize2.sh"
"#;
        let (_temp_dir, toml_path) = create_temp_backend_toml(content);
        let config = BackendConfiguration::read_backend_config(&toml_path).unwrap();

        assert!(config.config.contains_key("backend1"));
        assert!(config.config.contains_key("backend2"));

        let backend1 = config.get_backend(&"backend1".to_string()).unwrap();
        assert!(backend1.nixos.is_some());
        assert!(backend1.home.is_none());

        let backend2 = config.get_backend(&"backend2".to_string()).unwrap();
        assert!(backend2.nixos.is_some());
        assert!(backend2.home.is_some());
    }

    #[test]
    fn test_new_format_include_directive() {
        let temp_dir = TempDir::new().unwrap();

        let included_content = r#"
[included.nixos]
check = "./included_check.sh"
serialize = "./included_serialize.sh"
"#;
        let included_path = temp_dir.path().join("included.toml");
        let mut file = fs::File::create(&included_path).unwrap();
        file.write_all(included_content.as_bytes()).unwrap();

        let main_content = r#"
include = ["./included.toml"]

[main.nixos]
check = "./main_check.sh"
serialize = "./main_serialize.sh"
"#;
        let main_path = temp_dir.path().join("backend.toml");
        let mut file = fs::File::create(&main_path).unwrap();
        file.write_all(main_content.as_bytes()).unwrap();

        let config = BackendConfiguration::read_backend_config(&main_path).unwrap();

        assert!(config.config.contains_key("main"));
        assert!(config.config.contains_key("included"));
    }

    #[test]
    fn test_script_path_resolution_relative() {
        let content = r#"
[test.nixos]
check = "./check.sh"
serialize = "./serialize.sh"
"#;
        let temp_dir = TempDir::new().unwrap();
        let toml_path = temp_dir.path().join("backend.toml");
        let mut file = fs::File::create(&toml_path).unwrap();
        file.write_all(content.as_bytes()).unwrap();

        let config = BackendConfiguration::read_backend_config(&toml_path).unwrap();
        let backend = config.get_backend(&"test".to_string()).unwrap();

        let nixos = backend.nixos.unwrap();
        assert!(nixos.check.unwrap().ends_with("check.sh"));
    }

    #[test]
    fn test_target_config_serializes_true_with_both_scripts() {
        let target = TargetConfig {
            enabled: None,
            check: Some("./check.sh".to_string()),
            serialize: Some("./serialize.sh".to_string()),
        };
        assert!(target.serializes());
        assert!(target.is_enabled());
    }

    #[test]
    fn test_target_config_serializes_false_without_scripts() {
        let target = TargetConfig {
            enabled: Some(true),
            check: None,
            serialize: None,
        };
        assert!(!target.serializes());
        assert!(target.is_enabled());
    }

    #[test]
    fn test_target_config_disabled_explicit() {
        let target = TargetConfig {
            enabled: Some(false),
            check: Some("./check.sh".to_string()),
            serialize: Some("./serialize.sh".to_string()),
        };
        assert!(target.serializes());
        assert!(!target.is_enabled());
    }

    #[test]
    fn test_target_config_validation_success() {
        let target = TargetConfig {
            enabled: None,
            check: Some("./check.sh".to_string()),
            serialize: Some("./serialize.sh".to_string()),
        };
        assert!(target.validate(TargetType::NixOS, "test").is_ok());

        let target_passthrough = TargetConfig {
            enabled: Some(true),
            check: None,
            serialize: None,
        };
        assert!(
            target_passthrough
                .validate(TargetType::Home, "test")
                .is_ok()
        );
    }

    #[test]
    fn test_target_config_validation_check_without_serialize() {
        let target = TargetConfig {
            enabled: None,
            check: Some("./check.sh".to_string()),
            serialize: None,
        };
        let result = target.validate(TargetType::NixOS, "test");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("'check' requires 'serialize'")
        );
    }

    #[test]
    fn test_target_config_validation_serialize_without_check() {
        let target = TargetConfig {
            enabled: None,
            check: None,
            serialize: Some("./serialize.sh".to_string()),
        };
        let result = target.validate(TargetType::Home, "test");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("'serialize' requires 'check'")
        );
    }
}
