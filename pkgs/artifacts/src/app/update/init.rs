use super::super::effect::Effect;
use super::super::model::*;

/// Compute the initial effect to run when the app starts.
///
/// This triggers `check_serialization` for all pending artifacts,
/// determining which artifacts need regeneration before user interaction.
///
/// # Arguments
///
/// * `model` - The initial application model
///
/// # Returns
///
/// A batched [`Effect::CheckSerialization`] for all pending entries
pub fn init(model: &Model) -> Effect {
    let effects: Vec<Effect> = model
        .entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| entry.status() == &ArtifactStatus::Pending)
        .map(|(i, entry)| match entry {
            ListEntry::Single(single) => Effect::CheckSerialization {
                artifact_index: i,
                artifact_name: single.artifact.name.clone(),
                target_type: single.target_type.clone(),
            },
            ListEntry::Shared(shared) => Effect::SharedCheckSerialization {
                artifact_index: i,
                artifact_name: shared.info.artifact_name.clone(),
                nixos_targets: shared.info.nixos_targets.clone(),
                home_targets: shared.info.home_targets.clone(),
            },
        })
        .collect();
    Effect::batch(effects)
}
