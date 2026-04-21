//! Log types for artifact generation output.

use std::collections::HashSet;
use std::time::SystemTime;

/// A single log entry for an artifact generation step.
#[derive(Debug, Clone)]
pub struct LogEntry {
    /// Severity level of the log entry
    pub level: LogLevel,
    /// Log message content
    pub message: String,
}

/// Log severity/category for log entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    /// Summary messages
    Info,
    /// Stdout from generator/backend scripts
    Output,
    /// Stderr from generator/backend scripts
    Error,
    /// Completion/success messages
    Success,
}

/// Identifies which stream a line came from for streaming output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum OutputStream {
    /// Standard output from a script
    Stdout,
    /// Standard error from a script
    Stderr,
}

/// A warning about backend capability issues (non-blocking)
#[derive(Debug, Clone)]
pub struct Warning {
    pub artifact_name: String,
    pub message: String,
}

/// The three steps of the artifact generation pipeline.
///
/// Used both to track the currently-running step (via
/// [`GeneratingSubstate`](super::artifact::GeneratingSubstate)) and to
/// organise logs by phase in [`StepLogs`] and [`ChronologicalLogState`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum Step {
    #[default]
    Check,
    Generate,
    Serialize,
}

impl Step {
    pub fn next(self) -> Self {
        match self {
            Self::Check => Self::Generate,
            Self::Generate => Self::Serialize,
            Self::Serialize => Self::Check,
        }
    }

    pub fn previous(self) -> Self {
        match self {
            Self::Check => Self::Serialize,
            Self::Generate => Self::Check,
            Self::Serialize => Self::Generate,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Check => "Check",
            Self::Generate => "Generate",
            Self::Serialize => "Serialize",
        }
    }

    /// Get all possible step variants
    pub fn all_steps() -> &'static [Step] {
        static STEPS: &[Step] = &[Step::Check, Step::Generate, Step::Serialize];
        STEPS
    }
}

/// Logs organized by step
#[derive(Debug, Clone, Default)]
pub struct StepLogs {
    pub check: Vec<LogEntry>,
    pub generate: Vec<LogEntry>,
    pub serialize: Vec<LogEntry>,
}

impl StepLogs {
    pub fn get(&self, step: Step) -> &Vec<LogEntry> {
        match step {
            Step::Check => &self.check,
            Step::Generate => &self.generate,
            Step::Serialize => &self.serialize,
        }
    }

    pub fn get_mut(&mut self, step: Step) -> &mut Vec<LogEntry> {
        match step {
            Step::Check => &mut self.check,
            Step::Generate => &mut self.generate,
            Step::Serialize => &mut self.serialize,
        }
    }

    /// Append stdout lines as Output-level entries
    pub fn append_stdout(&mut self, step: Step, lines: &[String]) {
        let entries = lines.iter().map(|line| LogEntry {
            level: LogLevel::Output,
            message: line.clone(),
        });
        self.get_mut(step).extend(entries);
    }

    /// Append stderr lines as Error-level entries
    pub fn append_stderr(&mut self, step: Step, lines: &[String]) {
        let entries = lines.iter().map(|line| LogEntry {
            level: LogLevel::Error,
            message: line.clone(),
        });
        self.get_mut(step).extend(entries);
    }
}

/// A single execution pass for an artifact.
///
/// Each run bundles the [`StepLogs`] produced by one trip through the
/// check → generate → serialize pipeline. Entries keep a `Vec<GenerationRun>`
/// so reruns don't collapse into the same per-step vecs.
#[derive(Debug, Clone)]
pub struct GenerationRun {
    pub started_at: SystemTime,
    pub step_logs: StepLogs,
}

impl GenerationRun {
    pub fn new() -> Self {
        Self {
            started_at: SystemTime::now(),
            step_logs: StepLogs::default(),
        }
    }
}

impl Default for GenerationRun {
    fn default() -> Self {
        Self::new()
    }
}

/// Returns a shared empty [`StepLogs`] used when an entry has no runs yet.
///
/// Keeps `ListEntry::step_logs()` returning `&StepLogs` for views that still
/// read a flat per-step structure, without forcing every entry to carry an
/// initial run.
pub(crate) fn empty_step_logs() -> &'static StepLogs {
    use std::sync::OnceLock;
    static EMPTY: OnceLock<StepLogs> = OnceLock::new();
    EMPTY.get_or_init(StepLogs::default)
}

/// State for the chronological log view with expandable sections
#[derive(Debug, Clone)]
pub struct ChronologicalLogState {
    /// Index of the artifact being viewed
    pub artifact_index: usize,
    /// Artifact name for the header display
    pub artifact_name: String,
    /// Which sections are currently expanded (all steps by default)
    pub expanded_sections: HashSet<Step>,
    /// Vertical scroll offset in lines
    pub scroll_offset: usize,
    /// Currently focused section for keyboard navigation
    pub focused_section: Option<Step>,
}

impl ChronologicalLogState {
    /// Create a new chronological log state for viewing an artifact
    pub fn new(artifact_index: usize, artifact_name: String) -> Self {
        Self {
            artifact_index,
            artifact_name,
            expanded_sections: Step::all_steps().iter().cloned().collect(),
            scroll_offset: 0,
            focused_section: Some(Step::Check),
        }
    }

    /// Check if a specific section is expanded
    pub fn is_expanded(&self, step: Step) -> bool {
        self.expanded_sections.contains(&step)
    }

    /// Toggle a section's expanded state
    pub fn toggle_section(&mut self, step: Step) {
        if self.expanded_sections.contains(&step) {
            self.expanded_sections.remove(&step);
        } else {
            self.expanded_sections.insert(step);
        }
    }

    /// Expand all sections
    pub fn expand_all(&mut self) {
        self.expanded_sections = Step::all_steps().iter().cloned().collect();
    }

    /// Collapse all sections
    pub fn collapse_all(&mut self) {
        self.expanded_sections.clear();
    }

    /// Move focus to the next section
    pub fn focus_next(&mut self) {
        self.focused_section = self.focused_section.map(|s| s.next());
    }

    /// Move focus to the previous section
    pub fn focus_previous(&mut self) {
        self.focused_section = self.focused_section.map(|s| s.previous());
    }

    /// Scroll down by a number of lines
    pub fn scroll_down(&mut self, lines: usize) {
        self.scroll_offset += lines;
    }

    /// Scroll up by a number of lines
    pub fn scroll_up(&mut self, lines: usize) {
        if lines > self.scroll_offset {
            self.scroll_offset = 0;
        } else {
            self.scroll_offset -= lines;
        }
    }

    /// Calculate the maximum scroll offset based on content and visible height
    pub fn max_scroll(&self, step_logs: &StepLogs) -> usize {
        let mut total_lines = 0usize;
        for step in Step::all_steps() {
            // One line for the section header
            total_lines += 1;
            // If expanded, add log lines
            if self.is_expanded(*step) {
                total_lines += step_logs.get(*step).len();
            }
        }
        total_lines
    }

    /// Clamp scroll offset to valid range
    pub fn clamp_scroll(&mut self, max_scroll: usize) {
        self.scroll_offset = self.scroll_offset.min(max_scroll);
    }
}

impl Default for ChronologicalLogState {
    fn default() -> Self {
        // All sections are expanded by default
        Self {
            artifact_index: 0,
            artifact_name: String::new(),
            expanded_sections: Step::all_steps().iter().cloned().collect(),
            scroll_offset: 0,
            focused_section: Some(Step::Check),
        }
    }
}
