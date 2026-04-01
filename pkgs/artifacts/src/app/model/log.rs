//! Log types for artifact generation output.

use std::collections::HashSet;

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

/// The three generation steps that produce logs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum LogStep {
    #[default]
    Check,
    Generate,
    Serialize,
}

impl LogStep {
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
    pub fn all_steps() -> &'static [LogStep] {
        static STEPS: &[LogStep] = &[LogStep::Check, LogStep::Generate, LogStep::Serialize];
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
    pub fn get(&self, step: LogStep) -> &Vec<LogEntry> {
        match step {
            LogStep::Check => &self.check,
            LogStep::Generate => &self.generate,
            LogStep::Serialize => &self.serialize,
        }
    }

    pub fn get_mut(&mut self, step: LogStep) -> &mut Vec<LogEntry> {
        match step {
            LogStep::Check => &mut self.check,
            LogStep::Generate => &mut self.generate,
            LogStep::Serialize => &mut self.serialize,
        }
    }

    /// Append stdout lines as Output-level entries
    pub fn append_stdout(&mut self, step: LogStep, lines: &[String]) {
        let entries = lines.iter().map(|line| LogEntry {
            level: LogLevel::Output,
            message: line.clone(),
        });
        self.get_mut(step).extend(entries);
    }

    /// Append stderr lines as Error-level entries
    pub fn append_stderr(&mut self, step: LogStep, lines: &[String]) {
        let entries = lines.iter().map(|line| LogEntry {
            level: LogLevel::Error,
            message: line.clone(),
        });
        self.get_mut(step).extend(entries);
    }
}

/// State for the chronological log view with expandable sections
#[derive(Debug, Clone)]
pub struct ChronologicalLogState {
    /// Index of the artifact being viewed
    pub artifact_index: usize,
    /// Artifact name for the header display
    pub artifact_name: String,
    /// Which sections are currently expanded (all steps by default)
    pub expanded_sections: HashSet<LogStep>,
    /// Vertical scroll offset in lines
    pub scroll_offset: usize,
    /// Currently focused section for keyboard navigation
    pub focused_section: Option<LogStep>,
}

impl ChronologicalLogState {
    /// Create a new chronological log state for viewing an artifact
    pub fn new(artifact_index: usize, artifact_name: String) -> Self {
        Self {
            artifact_index,
            artifact_name,
            expanded_sections: LogStep::all_steps().iter().cloned().collect(),
            scroll_offset: 0,
            focused_section: Some(LogStep::Check),
        }
    }

    /// Check if a specific section is expanded
    pub fn is_expanded(&self, step: LogStep) -> bool {
        self.expanded_sections.contains(&step)
    }

    /// Toggle a section's expanded state
    pub fn toggle_section(&mut self, step: LogStep) {
        if self.expanded_sections.contains(&step) {
            self.expanded_sections.remove(&step);
        } else {
            self.expanded_sections.insert(step);
        }
    }

    /// Expand all sections
    pub fn expand_all(&mut self) {
        self.expanded_sections = LogStep::all_steps().iter().cloned().collect();
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
        for step in LogStep::all_steps() {
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
            expanded_sections: LogStep::all_steps().iter().cloned().collect(),
            scroll_offset: 0,
            focused_section: Some(LogStep::Check),
        }
    }
}
