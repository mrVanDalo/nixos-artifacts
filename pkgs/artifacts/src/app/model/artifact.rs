//! Artifact types and status definitions.

use super::log::StepLogs;
use super::target::TargetType;
use crate::config::make::{ArtifactDef, SharedArtifactInfo};
use ratatui::style::{Color, Style};

/// A per-target artifact entry (one machine or user).
///
/// Represents an artifact that belongs to a specific target (NixOS machine
/// or home-manager user). Each target has its own independent copy.
#[derive(Debug, Clone)]
pub struct ArtifactEntry {
    /// Type of target (NixOS machine or home-manager user) with name
    pub target_type: TargetType,
    /// The artifact definition (name, files, prompts, generator)
    pub artifact: ArtifactDef,
    /// Current generation status
    pub status: ArtifactStatus,
    /// Logs organized by generation step
    pub step_logs: StepLogs,
}

/// Typed error enum for artifact failures.
///
/// This enum represents all possible error conditions that can occur during
/// artifact processing, providing structured error information instead of
/// plain strings. This enables:
/// - Programmatic error handling (e.g., different retry behavior per type)
/// - Customized error display per type
/// - Type-safe error context preservation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactError {
    /// Script exceeded its timeout during execution.
    ScriptTimeout {
        /// Name of the script or operation that timed out
        script_name: String,
        /// Timeout duration in seconds
        timeout_secs: u64,
    },
    /// Script exited with non-zero exit code.
    ScriptFailed {
        /// Name of the script or operation that failed
        script_name: String,
        /// Exit code from the script
        exit_code: Option<i32>,
        /// Summary of stderr output (first ~200 chars)
        stderr_summary: String,
    },
    /// Generated files didn't match expected files.
    ValidationFailed {
        /// Description of what validation failed
        reason: String,
    },
    /// Artifact not found in configuration.
    ArtifactNotFound {
        /// Name of the artifact that wasn't found
        artifact_name: String,
        /// Target (machine/user) context
        target: String,
    },
    /// Task panicked during execution.
    TaskPanic {
        /// Panic message or description
        message: String,
    },
    /// I/O or other runtime error during execution.
    IoError {
        /// Context describing what operation failed
        context: String,
    },
    /// Configuration validation error (cannot be fixed by retry).
    ConfigurationError {
        /// Description of the configuration issue
        message: String,
    },
}

impl ArtifactError {
    /// Whether this error type supports retrying.
    ///
    /// Runtime errors (timeouts, script failures, I/O) can potentially
    /// succeed on retry. Configuration errors require user intervention.
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::ScriptTimeout { .. }
            | Self::ScriptFailed { .. }
            | Self::TaskPanic { .. }
            | Self::IoError { .. }
            | Self::ArtifactNotFound { .. } => true,
            Self::ValidationFailed { .. } | Self::ConfigurationError { .. } => false,
        }
    }

    /// Human-readable summary for the status line (short form).
    pub fn summary(&self) -> String {
        match self {
            Self::ScriptTimeout {
                script_name,
                timeout_secs,
            } => {
                format!("Timed out after {} seconds ({})", timeout_secs, script_name)
            }
            Self::ScriptFailed {
                script_name,
                exit_code,
                ..
            } => {
                if let Some(code) = exit_code {
                    format!("{} failed (exit code {})", script_name, code)
                } else {
                    format!("{} failed", script_name)
                }
            }
            Self::ValidationFailed { reason } => format!("Validation failed: {}", reason),
            Self::ArtifactNotFound {
                artifact_name,
                target,
            } => {
                format!(
                    "Artifact '{}' not found for target '{}'",
                    artifact_name, target
                )
            }
            Self::TaskPanic { message } => format!("Task panicked: {}", message),
            Self::IoError { context } => format!("I/O error: {}", context),
            Self::ConfigurationError { message } => message.clone(),
        }
    }

    /// Detailed message for the log view (includes full context).
    pub fn detail(&self) -> String {
        match self {
            Self::ScriptTimeout {
                script_name,
                timeout_secs,
            } => {
                format!(
                    "Script '{}' exceeded timeout of {} seconds and was terminated.",
                    script_name, timeout_secs
                )
            }
            Self::ScriptFailed {
                script_name,
                exit_code,
                stderr_summary,
            } => {
                let code_str = exit_code
                    .map(|c| format!(" with exit code {}", c))
                    .unwrap_or_default();
                if stderr_summary.is_empty() {
                    format!("{} failed{}", script_name, code_str)
                } else {
                    format!("{} failed{}:\n{}", script_name, code_str, stderr_summary)
                }
            }
            Self::ValidationFailed { reason } => {
                format!("Generated files validation failed: {}", reason)
            }
            Self::ArtifactNotFound {
                artifact_name,
                target,
            } => {
                format!(
                    "Artifact '{}' was not found in the configuration for target '{}'.",
                    artifact_name, target
                )
            }
            Self::TaskPanic { message } => {
                format!("Background task panicked: {}", message)
            }
            Self::IoError { context } => {
                format!("I/O operation failed: {}", context)
            }
            Self::ConfigurationError { message } => {
                format!("Configuration error: {}", message)
            }
        }
    }
}

impl std::fmt::Display for ArtifactError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.summary())
    }
}

/// Current status of an artifact through its lifecycle.
///
/// The normal flow is `Pending` → (`NeedsGeneration` | `UpToDate`), where the
/// branch is decided by the backend's `check_serialization` step.  If the user
/// then triggers generation the status moves to `Generating` while the
/// generator and serialization steps run, and finally settles back to
/// `UpToDate`.  Any step can fail and move the status to `Failed`.
///
/// # State Assumptions
///
/// - `NeedsGeneration`: The artifact does NOT exist in backend storage and
///   needs to be generated for the first time. The UI shows "Generate".
/// - `UpToDate`: The artifact EXISTS in backend storage and is current.
///   If the user triggers generation from this state, the UI shows
///   "Regenerate" since the artifact already exists.
/// - `Pending`: Initial state, the check has not run yet.
/// - `Generating`: A generation operation is in progress.
/// - `Failed`: A previous operation failed, can be retried.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum ArtifactStatus {
    /// Initial state before the backend check has run.  Every entry starts
    /// here; the check step transitions it to either `NeedsGeneration` or
    /// `UpToDate`.
    #[default]
    Pending,
    /// The backend's `check_serialization` determined that the serialized
    /// artifact is stale or missing and needs to be regenerated.
    ///
    /// This status implies the artifact does NOT exist in backend storage.
    /// When the user triggers generation, the UI should show "Generate".
    NeedsGeneration,
    /// The backend's `check_serialization` confirmed the serialized artifact
    /// is current, *or* generation and serialization have just completed
    /// successfully.
    ///
    /// This status implies the artifact EXISTS in backend storage.
    /// When the user triggers generation, the UI should show "Regenerate"
    /// since this will overwrite an existing artifact.
    UpToDate,
    /// The generator or serialization step is actively running for this
    /// artifact. The inner state tracks which step and any output.
    Generating(GeneratingSubstate),
    /// A step (check, generation, or serialization) failed.  The error
    /// and output are preserved for display.
    Failed {
        /// Typed error describing what went wrong
        error: ArtifactError,
        /// Formatted step logs from the generation process
        output: String,
    },
}

impl ArtifactStatus {
    /// Get the display symbol for this status
    pub fn symbol(&self) -> &'static str {
        match self {
            ArtifactStatus::Pending => "○",
            ArtifactStatus::NeedsGeneration => "!",
            ArtifactStatus::UpToDate => "✓",
            ArtifactStatus::Generating(_) => "⟳",
            ArtifactStatus::Failed { .. } => "✗",
        }
    }

    /// Get the display style (color) for this status
    pub fn style(&self) -> Style {
        match self {
            ArtifactStatus::Pending => Style::default().fg(Color::Gray),
            ArtifactStatus::NeedsGeneration => Style::default().fg(Color::Yellow),
            ArtifactStatus::UpToDate => Style::default().fg(Color::Green),
            ArtifactStatus::Generating(_) => Style::default().fg(Color::Cyan),
            ArtifactStatus::Failed { .. } => Style::default().fg(Color::Red),
        }
    }

    /// Check if this status is currently generating
    pub fn is_generating(&self) -> bool {
        matches!(self, ArtifactStatus::Generating(_))
    }

    /// Check if generation can be started from this status
    pub fn can_generate(&self) -> bool {
        matches!(
            self,
            ArtifactStatus::Pending
                | ArtifactStatus::NeedsGeneration
                | ArtifactStatus::Failed { .. }
        )
    }
}

/// Substate while an artifact is being generated.
/// Tracks the current step and accumulated output for display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratingSubstate {
    /// Which step is currently running
    pub step: GenerationStep,
    /// Accumulated output from the generator (shown after completion)
    pub output: String,
}

impl Default for GeneratingSubstate {
    fn default() -> Self {
        Self {
            step: GenerationStep::CheckSerialization,
            output: String::new(),
        }
    }
}

/// The steps in the artifact generation process.
///
/// These steps are executed in order:
/// 1. CheckSerialization - Determine if regeneration is needed
/// 2. RunningGenerator - Execute the generator script
/// 3. Serializing - Store generated files in the backend
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GenerationStep {
    /// Checking if generation is needed via check_serialization script
    #[default]
    CheckSerialization,
    /// Running the generator script to produce files
    RunningGenerator,
    /// Running the serialize script to store files in backend
    Serializing,
}

impl GenerationStep {
    /// Get a human-readable description of this step
    pub fn description(&self) -> &'static str {
        match self {
            GenerationStep::CheckSerialization => "CheckSerialization...",
            GenerationStep::RunningGenerator => "Running generator...",
            GenerationStep::Serializing => "Serializing...",
        }
    }
}

/// An entry in the artifact list that can be either per-target or shared.
///
/// This enum unifies the artifact list display, handling both:
/// - Single artifacts (one per target)
/// - Shared artifacts (one artifact shared across multiple targets)
#[derive(Debug, Clone)]
pub enum ListEntry {
    /// Per-target artifact (one machine or user)
    Single(ArtifactEntry),
    /// Shared artifact across multiple targets
    Shared(SharedEntry),
}

impl ListEntry {
    pub fn artifact_name(&self) -> &str {
        match self {
            ListEntry::Single(entry) => &entry.artifact.name,
            ListEntry::Shared(entry) => &entry.info.artifact_name,
        }
    }

    pub fn status(&self) -> &ArtifactStatus {
        match self {
            ListEntry::Single(entry) => &entry.status,
            ListEntry::Shared(entry) => &entry.status,
        }
    }

    pub fn status_mut(&mut self) -> &mut ArtifactStatus {
        match self {
            ListEntry::Single(entry) => &mut entry.status,
            ListEntry::Shared(entry) => &mut entry.status,
        }
    }

    pub fn step_logs(&self) -> &StepLogs {
        match self {
            ListEntry::Single(entry) => &entry.step_logs,
            ListEntry::Shared(entry) => &entry.step_logs,
        }
    }

    pub fn step_logs_mut(&mut self) -> &mut StepLogs {
        match self {
            ListEntry::Single(entry) => &mut entry.step_logs,
            ListEntry::Shared(entry) => &mut entry.step_logs,
        }
    }

    pub fn is_shared(&self) -> bool {
        matches!(self, ListEntry::Shared(_))
    }

    pub fn target_type(&self) -> Option<&TargetType> {
        match self {
            ListEntry::Single(entry) => Some(&entry.target_type),
            ListEntry::Shared(_) => None,
        }
    }
}

/// A shared artifact entry in the list
#[derive(Debug, Clone)]
pub struct SharedEntry {
    /// Shared artifact info (artifact name, generators, backend, prompts, files)
    pub info: SharedArtifactInfo,
    pub status: ArtifactStatus,
    pub step_logs: StepLogs,
    /// The selected generator path (set after user selection)
    pub selected_generator: Option<String>,
}
