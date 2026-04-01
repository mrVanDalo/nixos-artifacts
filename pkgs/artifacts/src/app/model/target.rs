//! Target type definitions for artifact entries.

/// Target type for single artifact entries.
///
/// Determines the context (NixOS vs home-manager) and affects
/// how artifacts are serialized and which environment variables
/// are passed to scripts. Shared artifacts are handled separately
/// via `SharedEntry` which contains target lists.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TargetType {
    /// NixOS machine configuration
    NixOS { machine: String },
    /// Home-manager user configuration
    HomeManager { username: String },
}

impl TargetType {
    pub fn context_str(&self) -> &'static str {
        match self {
            Self::NixOS { .. } => "nixos",
            Self::HomeManager { .. } => "homemanager",
        }
    }

    pub fn target_name(&self) -> &str {
        match self {
            Self::NixOS { machine } => machine,
            Self::HomeManager { username } => username,
        }
    }
}

impl std::fmt::Display for TargetType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.context_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_type_context_str() {
        assert_eq!(
            TargetType::NixOS {
                machine: "test".to_string()
            }
            .context_str(),
            "nixos"
        );
        assert_eq!(
            TargetType::HomeManager {
                username: "test".to_string()
            }
            .context_str(),
            "homemanager"
        );
    }

    #[test]
    fn test_target_type_target_name() {
        let nixos = TargetType::NixOS {
            machine: "my-machine".to_string(),
        };
        assert_eq!(nixos.target_name(), "my-machine");

        let home = TargetType::HomeManager {
            username: "alice@host".to_string(),
        };
        assert_eq!(home.target_name(), "alice@host");
    }
}
