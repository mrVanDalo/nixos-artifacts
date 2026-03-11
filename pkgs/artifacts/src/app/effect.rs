//! Side effect descriptors for the Elm Architecture runtime.
//!
//! This module defines the `Effect` enum - descriptions of side effects
//! that the runtime executes. Effects are returned by the pure `update`
//! function, keeping state transitions testable and predictable.
//!
//! # Effect Execution
//!
//! Effects are executed by the runtime (see [`crate::tui::runtime`]) which
//! spawns background tasks to run scripts. Results are fed back into the
//! update loop as messages.
//!
//! # Design Pattern
//!
//! Effects are data, not actions. The `update` function describes what
//! should happen (e.g., "run this generator"), and the runtime handles
//! the actual execution asynchronously.
//!
//! [`crate::tui::runtime`]: crate::tui::runtime

use std::collections::HashMap;

use crate::app::model::TargetType;

/// Side effects that the runtime must execute.
///
/// These are returned by [`update`](crate::app::update::update), not executed inside it.
/// The runtime sends these directly to the background task for execution.
#[derive(Debug, Clone, Default)]
pub enum Effect {
    /// No side effect
    #[default]
    None,

    /// Multiple effects to execute
    Batch(Vec<Self>),

    /// Quit the application
    Quit,

    /// Check if an artifact needs regeneration
    CheckSerialization {
        artifact_index: usize,
        artifact_name: String,
        target_type: TargetType,
    },

    /// Run the generator script for an artifact
    RunGenerator {
        artifact_index: usize,
        artifact_name: String,
        target_type: TargetType,
        prompts: HashMap<String, String>,
    },

    /// Serialize the generated files
    Serialize {
        artifact_index: usize,
        artifact_name: String,
        target_type: TargetType,
    },

    /// Check if a shared artifact needs regeneration
    SharedCheckSerialization {
        artifact_index: usize,
        artifact_name: String,
        nixos_targets: Vec<String>,
        home_targets: Vec<String>,
    },

    /// Run the generator script for a shared artifact
    RunSharedGenerator {
        artifact_index: usize,
        artifact_name: String,
        prompts: HashMap<String, String>,
    },

    /// Serialize the generated files for a shared artifact
    SharedSerialize {
        artifact_index: usize,
        artifact_name: String,
        nixos_targets: Vec<String>,
        home_targets: Vec<String>,
    },
}

impl Effect {
    /// Create a batch effect from an iterator of effects.
    /// Returns None if empty, the single effect if one, or Batch if multiple.
    pub fn batch(effects: impl IntoIterator<Item = Self>) -> Self {
        let v: Vec<_> = effects
            .into_iter()
            .filter(|e| !matches!(e, Self::None))
            .collect();
        match v.len() {
            0 => Self::None,
            1 => v.into_iter().next().unwrap(),
            _ => Self::Batch(v),
        }
    }

    /// Check if this is the None variant
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    /// Check if this is the Quit variant
    pub fn is_quit(&self) -> bool {
        matches!(self, Self::Quit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_empty() {
        let effect = Effect::batch(vec![]);
        assert!(effect.is_none());
    }

    #[test]
    fn test_batch_single() {
        let effect = Effect::batch(vec![Effect::Quit]);
        assert!(effect.is_quit());
    }

    #[test]
    fn test_batch_filters_none() {
        let effect = Effect::batch(vec![Effect::None, Effect::Quit, Effect::None]);
        assert!(effect.is_quit());
    }

    #[test]
    fn test_batch_multiple() {
        let effect = Effect::batch(vec![Effect::Quit, Effect::Quit]);
        assert!(matches!(effect, Effect::Batch(_)));
    }
}
