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

/// Identifies a focusable element in the chronological log view.
///
/// The view is a Run → Step tree: each run has its own header and can
/// contain the three step sub-sections. Focus targets either a whole run
/// or a specific step inside one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LogFocus {
    /// Focus on a run header (toggling expands/collapses the run)
    Run(usize),
    /// Focus on a step inside a run (toggling expands/collapses the step)
    Step(usize, Step),
}

/// State for the chronological log view with Run → Step collapsible hierarchy.
#[derive(Debug, Clone, Default)]
pub struct ChronologicalLogState {
    /// Index of the artifact being viewed
    pub artifact_index: usize,
    /// Artifact name for the header display
    pub artifact_name: String,
    /// Run indices that are currently expanded
    pub expanded_runs: HashSet<usize>,
    /// (run, step) pairs that are currently expanded (only visible when the
    /// enclosing run is also expanded)
    pub expanded_steps: HashSet<(usize, Step)>,
    /// Vertical scroll offset in lines
    pub scroll_offset: usize,
    /// Currently focused element (run header or step within a run)
    pub focus: Option<LogFocus>,
}

impl ChronologicalLogState {
    /// Create a new chronological log state for viewing an artifact.
    ///
    /// Default expand policy: only the latest run is expanded, with all of
    /// its steps expanded. Older runs are collapsed so the view stays
    /// scannable when an artifact has many runs.
    pub fn new(artifact_index: usize, artifact_name: String, num_runs: usize) -> Self {
        let mut expanded_runs = HashSet::new();
        let mut expanded_steps = HashSet::new();
        let focus = if num_runs == 0 {
            None
        } else {
            let latest = num_runs - 1;
            expanded_runs.insert(latest);
            for step in Step::all_steps() {
                expanded_steps.insert((latest, *step));
            }
            Some(LogFocus::Step(latest, Step::Check))
        };

        Self {
            artifact_index,
            artifact_name,
            expanded_runs,
            expanded_steps,
            scroll_offset: 0,
            focus,
        }
    }

    pub fn is_run_expanded(&self, run: usize) -> bool {
        self.expanded_runs.contains(&run)
    }

    pub fn is_step_expanded(&self, run: usize, step: Step) -> bool {
        self.expanded_steps.contains(&(run, step))
    }

    /// Toggle the focused element. Runs toggle their own expansion; steps
    /// toggle their own expansion regardless of the enclosing run state.
    pub fn toggle_focused(&mut self) {
        match self.focus {
            Some(LogFocus::Run(run)) => {
                if self.expanded_runs.contains(&run) {
                    self.expanded_runs.remove(&run);
                } else {
                    self.expanded_runs.insert(run);
                }
            }
            Some(LogFocus::Step(run, step)) => {
                let key = (run, step);
                if self.expanded_steps.contains(&key) {
                    self.expanded_steps.remove(&key);
                } else {
                    self.expanded_steps.insert(key);
                }
            }
            None => {}
        }
    }

    /// Expand every run and every step in the given run count.
    pub fn expand_all(&mut self, num_runs: usize) {
        self.expanded_runs = (0..num_runs).collect();
        self.expanded_steps = (0..num_runs)
            .flat_map(|run| Step::all_steps().iter().map(move |step| (run, *step)))
            .collect();
    }

    /// Collapse every run and every step.
    pub fn collapse_all(&mut self) {
        self.expanded_runs.clear();
        self.expanded_steps.clear();
    }

    /// Visible focusable targets in top-to-bottom order.
    fn visible_focus_order(&self, num_runs: usize) -> Vec<LogFocus> {
        let mut order = Vec::new();
        for run in 0..num_runs {
            order.push(LogFocus::Run(run));
            if self.is_run_expanded(run) {
                for step in Step::all_steps() {
                    order.push(LogFocus::Step(run, *step));
                }
            }
        }
        order
    }

    /// Move focus to the next visible target (wraps to start).
    pub fn focus_next(&mut self, num_runs: usize) {
        let order = self.visible_focus_order(num_runs);
        if order.is_empty() {
            self.focus = None;
            return;
        }
        let current = self
            .focus
            .and_then(|f| order.iter().position(|o| *o == f))
            .unwrap_or(order.len().saturating_sub(1));
        let next = (current + 1) % order.len();
        self.focus = Some(order[next]);
    }

    /// Move focus to the previous visible target (wraps to end).
    pub fn focus_previous(&mut self, num_runs: usize) {
        let order = self.visible_focus_order(num_runs);
        if order.is_empty() {
            self.focus = None;
            return;
        }
        let current = self
            .focus
            .and_then(|f| order.iter().position(|o| *o == f))
            .unwrap_or(0);
        let prev = if current == 0 {
            order.len() - 1
        } else {
            current - 1
        };
        self.focus = Some(order[prev]);
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

    /// Estimate the total rendered line count for scroll clamping.
    pub fn max_scroll(&self, runs: &[GenerationRun]) -> usize {
        let mut total_lines = 0usize;
        for (idx, run) in runs.iter().enumerate() {
            // Run header line
            total_lines += 1;
            if !self.is_run_expanded(idx) {
                continue;
            }
            for step in Step::all_steps() {
                // Step header line
                total_lines += 1;
                if self.is_step_expanded(idx, *step) {
                    total_lines += run.step_logs.get(*step).len();
                }
            }
        }
        total_lines
    }

    /// Clamp scroll offset to valid range
    pub fn clamp_scroll(&mut self, max_scroll: usize) {
        self.scroll_offset = self.scroll_offset.min(max_scroll);
    }
}
