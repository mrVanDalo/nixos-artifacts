use std::collections::HashMap;
use std::path::PathBuf;

/// Side effects that the runtime must execute.
/// These are returned by update(), not executed inside it.
#[derive(Debug, Clone)]
pub enum Effect {
    /// No side effect
    None,

    /// Multiple effects to execute
    Batch(Vec<Effect>),

    /// Quit the application
    Quit,

    /// Check if an artifact needs regeneration
    CheckSerialization {
        artifact_index: usize,
        artifact_name: String,
        target: String,
        target_type: crate::app::model::TargetType,
    },

    /// Run the generator script for an artifact
    RunGenerator {
        artifact_index: usize,
        artifact_name: String,
        target: String,
        target_type: crate::app::model::TargetType,
        prompts: HashMap<String, String>,
    },

    /// Serialize the generated files
    Serialize {
        artifact_index: usize,
        artifact_name: String,
        target: String,
        target_type: crate::app::model::TargetType,
        out_dir: PathBuf,
    },
}

impl Effect {
    /// Create a batch effect from an iterator of effects.
    /// Returns None if empty, the single effect if one, or Batch if multiple.
    pub fn batch(effects: impl IntoIterator<Item = Effect>) -> Self {
        let v: Vec<_> = effects
            .into_iter()
            .filter(|e| !matches!(e, Effect::None))
            .collect();
        match v.len() {
            0 => Effect::None,
            1 => v.into_iter().next().unwrap(),
            _ => Effect::Batch(v),
        }
    }

    /// Check if this is the None variant
    pub fn is_none(&self) -> bool {
        matches!(self, Effect::None)
    }

    /// Check if this is the Quit variant
    pub fn is_quit(&self) -> bool {
        matches!(self, Effect::Quit)
    }
}

impl Default for Effect {
    fn default() -> Self {
        Self::None
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
