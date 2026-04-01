//! Screen-specific state types for dialogs and generation views.

use super::artifact::GenerationStep;
use crate::config::make::{GeneratorInfo, PromptDef};

/// State while generating an artifact (screen state)
#[derive(Debug, Clone)]
pub struct GeneratingState {
    pub artifact_index: usize,
    pub artifact_name: String,
    pub step: GenerationStep,
    pub log_lines: Vec<String>,
    /// true if regenerating existing artifact, false if creating new
    pub exists: bool,
}

/// State when generation is complete
#[derive(Debug, Clone, Default)]
pub struct DoneState {
    pub generated_count: usize,
    pub skipped_count: usize,
    pub failed: Vec<String>,
}

/// State for the regeneration confirmation dialog
#[derive(Debug, Clone)]
pub struct ConfirmRegenerateState {
    pub artifact_index: usize,
    pub artifact_name: String,
    /// Description of affected targets (for shared artifacts)
    pub affected_targets: Vec<String>,
    /// true = Leave selected (safe), false = Regenerate selected
    pub leave_selected: bool,
}

/// State for the generator selection screen (for shared artifacts with multiple generators)
#[derive(Debug, Clone)]
pub struct SelectGeneratorState {
    pub artifact_index: usize,
    pub artifact_name: String,
    /// artifact description (optional, for display in dialog)
    pub description: Option<String>,
    pub generators: Vec<GeneratorInfo>,
    pub selected_index: usize,
    /// Prompts required for this artifact (from config::make::PromptDef)
    pub prompts: Vec<PromptDef>,
    /// NixOS machine names that use this artifact
    pub nixos_targets: Vec<String>,
    /// Home-manager user identifiers that use this artifact
    pub home_targets: Vec<String>,
}

impl SelectGeneratorState {
    pub fn selected_generator(&self) -> Option<&GeneratorInfo> {
        self.generators.get(self.selected_index)
    }
}
