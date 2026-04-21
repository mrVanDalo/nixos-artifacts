use super::super::effect::{Effect, TargetSpec};
use super::super::model::*;

/// Compute the initial effect to run when the app starts.
///
/// This triggers `check_serialization` for all pending artifacts,
/// determining which artifacts need regeneration before user interaction.
/// Each check also starts a fresh [`super::super::model::GenerationRun`] on
/// its entry so subsequent log output lands in a dedicated run bucket.
///
/// # Arguments
///
/// * `model` - The initial application model (mutated to seed runs)
///
/// # Returns
///
/// A batched [`Effect::CheckSerialization`] for all pending entries
pub fn init(model: &mut Model) -> Effect {
    let pending_indices: Vec<usize> = model
        .entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| entry.status() == &ArtifactStatus::Pending)
        .map(|(i, _)| i)
        .collect();

    let effects: Vec<Effect> = pending_indices
        .into_iter()
        .filter_map(|i| {
            let entry = model.entries.get_mut(i)?;
            entry.start_new_run();
            let effect = match entry {
                ListEntry::Single(single) => Effect::CheckSerialization {
                    artifact_index: i,
                    artifact_name: single.artifact.name.clone(),
                    target_spec: TargetSpec::Single(single.target_type.clone()),
                },
                ListEntry::Shared(shared) => Effect::CheckSerialization {
                    artifact_index: i,
                    artifact_name: shared.info.artifact_name.clone(),
                    target_spec: TargetSpec::Multi {
                        nixos_targets: shared.info.nixos_targets.clone(),
                        home_targets: shared.info.home_targets.clone(),
                    },
                },
            };
            Some(effect)
        })
        .collect();

    Effect::batch(effects)
}
