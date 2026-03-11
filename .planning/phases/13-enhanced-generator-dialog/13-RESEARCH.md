# Phase 13: Enhanced Generator Dialog - Research

**Researched:** 2026-02-18\
**Domain:** Rust TUI (ratatui), Terminal UI Layout\
**Confidence:** HIGH

## Summary

The Enhanced Generator Dialog phase focuses on improving the presentation of the
existing generator selection dialog in the TUI. The dialog currently shows basic
generator paths and their associated machines/users. This phase adds rich
context: artifact descriptions, prompt descriptions, shared vs per-machine
indicators, and comprehensive machine/user listings.

**Primary recommendation:** Extend `SelectGeneratorState` with artifact
metadata, restructure `render_generator_selection()` to use a multi-section
layout with clear visual hierarchy, maintain existing list navigation behavior,
and add comprehensive snapshot tests for the new layout variations.

## User Constraints (from CONTEXT.md)

### Locked Decisions

**Layout and information hierarchy:**

- Artifact type indicator (shared vs per-machine) appears at the very top
- Artifact name appears in the dialog title/header itself
- Artifact description appears above the generator list (if available)
- Prompt descriptions appear in a dedicated section with clear labels, displayed
  before the generator selection
- Machine/user info appears at the bottom of the dialog
- Sections are visually separated by line separators
- Machines/users are alphabetically sorted

**Visual presentation style:**

- Shared vs per-machine status indicated with text labels only (no color-coding)
- NixOS machines vs home-manager distinguished by type prefix: `nixos:` and
  `home:`
- Long generator paths (Nix store paths) are truncated with ellipsis
- Currently selected generator indicated with arrow `>` symbol
- Prompt descriptions displayed as labeled list items (e.g., "Prompt 1:
  Description")

**Content display:**

- Long prompt descriptions show full text (no truncation)
- If artifact has no description defined: show fallback text "No description
  provided"
- Generator context (machine/user info) formatted consistently showing full
  identifier (machine name for NixOS, user@host for home-manager)

**Machine/user listing format:**

- Each machine/user displayed on its own line (vertical list)
- If more than 10 machines: show first 10 with "+N more" indicator

### Claude's Discretion

- Exact spacing and padding between sections
- Whether to use bold/italic text for labels vs values
- Specific character limit for truncated paths
- Exact formatting of the "+N more" indicator text

### Deferred Ideas (OUT OF SCOPE)

- None — discussion stayed within phase scope

---

## Standard Stack

### Core

| Library   | Version | Purpose               | Why Standard                                               |
| --------- | ------- | --------------------- | ---------------------------------------------------------- |
| ratatui   | 0.29    | Terminal UI framework | Established Rust TUI library with widgets, layout, styling |
| crossterm | 0.28    | Terminal manipulation | Cross-platform terminal handling, works with ratatui       |
| insta     | 1.43.1  | Snapshot testing      | Used throughout codebase for view tests                    |

### Widgets Used in Current Dialog

| Widget          | Purpose                               |
| --------------- | ------------------------------------- |
| `Block`         | Container with borders and title      |
| `List`          | Generator selection with highlighting |
| `ListItem`      | Individual generator entries          |
| `Line` / `Span` | Text styling and composition          |
| `Paragraph`     | Multi-line text display               |

### Layout System

```rust
// Standard ratatui layout pattern used in codebase
use ratatui::layout::{Constraint, Layout, Rect};

// Vertical stacking with fixed and flexible sections
let chunks = Layout::vertical([
    Constraint::Length(3),  // Fixed: Header
    Constraint::Min(1),     // Flexible: Content
    Constraint::Length(2),  // Fixed: Help
]).split(area);
```

---

## Architecture Patterns

### Current Dialog Structure (Model)

```rust
// src/app/model.rs - SelectGeneratorState (existing)
pub struct SelectGeneratorState {
    pub artifact_index: usize,
    pub artifact_name: String,
    pub generators: Vec<GeneratorInfo>,
    pub selected_index: usize,
}

// GeneratorInfo from config::make
pub struct GeneratorInfo {
    pub path: String,
    pub sources: Vec<GeneratorSource>,
}

pub struct GeneratorSource {
    pub target: String,           // machine name or user@host
    pub target_type: TargetType,  // Nixos or HomeManager
}
```

### Extended Model for Enhanced Dialog

**Required additions to SelectGeneratorState:**

```rust
pub struct SelectGeneratorState {
    pub artifact_index: usize,
    pub artifact_name: String,
    pub artifact_description: Option<String>,  // NEW
    pub is_shared: bool,                        // NEW (always true for this dialog)
    pub prompts: Vec<PromptEntry>,              // NEW - from SharedArtifactInfo.prompts
    pub nixos_targets: Vec<String>,             // NEW - all NixOS machines
    pub home_targets: Vec<String>,              // NEW - all home-manager users
    pub generators: Vec<GeneratorInfo>,
    pub selected_index: usize,
}
```

### View Layout Pattern (Recommended)

```rust
// Enhanced generator selection layout
pub fn render_generator_selection(frame: &mut Frame, state: &SelectGeneratorState, area: Rect) {
    // Split area into sections
    let chunks = Layout::vertical([
        Constraint::Length(3),      // Title block
        Constraint::Length(2),      // Type indicator (shared/per-machine)
        Constraint::Length(desc_height), // Description (variable)
        Constraint::Length(prompts_height), // Prompts section (variable)
        Constraint::Length(1),      // Separator line
        Constraint::Min(5),         // Generator list (flexible)
        Constraint::Length(1),      // Separator line
        Constraint::Length(targets_height), // All targets list
        Constraint::Length(2),      // Help text
    ]).split(area);
    
    // Render each section
    render_title_block(frame, state, chunks[0]);
    render_type_indicator(frame, state, chunks[1]);
    render_description(frame, state, chunks[2]);
    render_prompts_section(frame, state, chunks[3]);
    // ... generator list (existing logic adapted)
    render_all_targets(frame, state, chunks[6]);
    render_help(frame, state, chunks[7]);
}
```

### Data Flow from Config to Dialog

```rust
// From SharedArtifactInfo (src/config/make.rs)
// Available data for the dialog:
- artifact_name: String
- generators: Vec<GeneratorInfo>
- nixos_targets: Vec<String>    // All machines using this artifact
- home_targets: Vec<String>     // All users using this artifact
- prompts: BTreeMap<String, PromptDef>  // Prompt descriptions
- files: BTreeMap<String, FileDef>
- error: Option<String>

// PromptDef contains:
- name: String
- description: Option<String>
```

### Current Rendering Pattern (From Codebase)

```rust
// src/tui/views/generator_selection.rs - Current implementation
pub fn render_generator_selection(frame: &mut Frame, state: &SelectGeneratorState, area: Rect) {
    let mut items: Vec<ListItem> = Vec::new();
    
    for (idx, gen_info) in state.generators.iter().enumerate() {
        let is_selected = idx == state.selected_index;
        
        // Path line with styling
        let path_style = if is_selected { 
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else { 
            Style::default().fg(Color::White)
        };
        
        // Source counts
        let count_summary = format_source_counts(nixos_count, home_count);
        
        // Build list item
        items.push(ListItem::new(Line::from(vec![
            Span::styled(&gen_info.path, path_style),
            Span::styled(" ", Style::default()),
            Span::styled(count_summary, Style::default().fg(Color::DarkGray)),
        ])));
        
        // Source details with tree characters
        for (source_idx, source) in gen_info.sources.iter().enumerate() {
            let is_last = source_idx == source_count - 1;
            let tree_char = if is_last { "└─" } else { "├─" };
            
            let (type_label, type_color) = match source.target_type {
                TargetType::Nixos => ("NixOS", Color::Blue),
                TargetType::HomeManager => ("home-manager", Color::Magenta),
            };
            
            items.push(ListItem::new(Line::from(vec![
                Span::styled("    ", Style::default().fg(Color::DarkGray)),
                Span::styled(tree_char, Style::default().fg(Color::DarkGray)),
                Span::styled(type_label, Style::default().fg(type_color).add_modifier(Modifier::BOLD)),
                Span::styled(": ", Style::default().fg(Color::DarkGray)),
                Span::styled(&source.target, Style::default().fg(Color::White)),
            ])));
        }
    }
    
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD))
        .highlight_symbol("> ");
    
    frame.render_stateful_widget(list, area, &mut list_state);
}
```

### Text Truncation Pattern

```rust
// Pattern for truncating long paths (Nix store paths)
fn truncate_path(path: &str, max_len: usize) -> String {
    if path.len() <= max_len {
        path.to_string()
    } else {
        format!("...{}", &path[path.len() - max_len + 3..])
    }
}

// Alternative: truncate from middle
fn truncate_path_middle(path: &str, max_len: usize) -> String {
    if path.len() <= max_len {
        path.to_string()
    } else {
        let keep_start = (max_len - 3) / 2;
        let keep_end = (max_len - 3) - keep_start;
        format!("{}...{}", &path[..keep_start], &path[path.len() - keep_end..])
    }
}
```

---

## Don't Hand-Roll

| Problem                    | Don't Build                  | Use Instead                        | Why                                                         |
| -------------------------- | ---------------------------- | ---------------------------------- | ----------------------------------------------------------- |
| Terminal UI framework      | Custom ANSI escape sequences | ratatui                            | Event handling, widgets, layout, async support all built-in |
| Text wrapping              | Manual string splitting      | ratatui `Paragraph` with `Wrap`    | Handles Unicode, terminal width, edge cases                 |
| Terminal backend detection | Direct termios calls         | crossterm                          | Cross-platform (Windows, macOS, Linux)                      |
| Snapshot testing           | File-based comparison        | insta crate                        | Structured diff, inline snapshots, review workflow          |
| Layout calculation         | Manual coordinate math       | ratatui `Layout` with `Constraint` | Responsive to terminal resize, percentage/flex support      |

---

## Common Pitfalls

### Pitfall 1: Breaking Existing Navigation

**What goes wrong:** Restructuring the layout breaks the visual index
calculation for list navigation.

**Why it happens:** The current `calculate_visual_index()` function assumes a
specific structure (generator lines + source lines + blank separators).

**How to avoid:**

- Keep generator list as a contiguous `List` widget for proper selection
  handling
- Move static context sections (description, prompts, targets) outside the
  `List`
- Use nested layouts: outer vertical split separates context from interactive
  list

```rust
// Safe layout structure
let main_chunks = Layout::vertical([
    Constraint::Length(context_height),  // Static: type, desc, prompts
    Constraint::Min(1),                   // Interactive: generator list
    Constraint::Length(targets_height),  // Static: all targets
]).split(area);

// Render list in its own area (preserves navigation)
render_generator_list(frame, state, main_chunks[1]);
```

### Pitfall 2: Overflow with Many Targets

**What goes wrong:** Dialog exceeds terminal height when an artifact is used by
many machines.

**Why it happens:** No limit on displayed targets; list grows indefinitely.

**How to avoid:**

- Implement the "+N more" truncation as specified
- Use `area.height` to dynamically adjust visible content
- Consider scrollable sections for extreme cases (>20 targets)

### Pitfall 3: Missing Description Field

**What goes wrong:** `ArtifactDef` has no `description` field currently.

**Why it happens:** The field needs to be added to both the Nix module and Rust
config parsing.

**How to avoid:**

- Phase 13 includes adding the description field to data models
- Must update: `modules/store.nix` → `ArtifactDef` in Rust →
  `SelectGeneratorState`
- Default to "No description provided" when absent

### Pitfall 4: Inconsistent Styling

**What goes wrong:** New sections use different color schemes than existing UI.

**Why it happens:** No established style guide in codebase; developers use
personal preference.

**How to avoid:**

- Follow existing patterns from `generator_selection.rs`:
  - Labels: `Color::DarkGray` with optional `Modifier::BOLD`
  - Values: `Color::White` or `Color::Cyan` for emphasis
  - Type indicators: `Color::Blue` for NixOS, `Color::Magenta` for home-manager
  - Selected items: `Color::Cyan` + `Modifier::BOLD` with `Color::DarkGray`
    background

### Pitfall 5: Test Snapshot Drift

**What goes wrong:** Snapshot tests fail after minor layout tweaks.

**Why it happens:** Snapshots capture exact terminal output including spacing.

**Warning signs:**

- Many snapshot files updated in single commit
- Test failures on unchanged functionality

**How to avoid:**

- Run `cargo insta review` (NEVER `cargo insta accept`)
- Review each diff carefully
- Use `TestBackend::new(width, height)` with consistent dimensions

---

## Code Examples

### Enhanced State Construction

```rust
// In model_builder.rs or where SelectGeneratorState is created
impl SelectGeneratorState {
    pub fn from_shared_entry(
        artifact_index: usize,
        shared: &SharedEntry,
    ) -> Self {
        let prompts: Vec<PromptEntry> = shared
            .info
            .prompts
            .values()
            .map(|p| PromptEntry {
                name: p.name.clone(),
                description: p.description.clone(),
            })
            .collect();
        
        // Sort targets alphabetically as required
        let mut nixos_targets = shared.info.nixos_targets.clone();
        let mut home_targets = shared.info.home_targets.clone();
        nixos_targets.sort();
        home_targets.sort();
        
        Self {
            artifact_index,
            artifact_name: shared.info.artifact_name.clone(),
            artifact_description: None, // TODO: Add to ArtifactDef
            is_shared: true,
            prompts,
            nixos_targets,
            home_targets,
            generators: shared.info.generators.clone(),
            selected_index: 0,
        }
    }
}
```

### Section Rendering Functions

```rust
// Type indicator at top (shared vs per-machine)
fn render_type_indicator(frame: &mut Frame, state: &SelectGeneratorState, area: Rect) {
    let text = if state.is_shared {
        "Type: Shared across machines"
    } else {
        "Type: Per-machine artifact"
    };
    
    let paragraph = Paragraph::new(text)
        .style(Style::default().fg(Color::White));
    
    frame.render_widget(paragraph, area);
}

// Description section
fn render_description(frame: &mut Frame, state: &SelectGeneratorState, area: Rect) {
    let desc = state.artifact_description.as_deref()
        .unwrap_or("No description provided");
    
    let lines = vec![
        Line::styled("Description:", Style::default().add_modifier(Modifier::BOLD)),
        Line::from(desc),
    ];
    
    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, area);
}

// Prompts section
fn render_prompts_section(frame: &mut Frame, state: &SelectGeneratorState, area: Rect) {
    if state.prompts.is_empty() {
        return;
    }
    
    let mut lines = vec![
        Line::styled("Prompts:", Style::default().add_modifier(Modifier::BOLD))
    ];
    
    for (idx, prompt) in state.prompts.iter().enumerate() {
        let desc = prompt.description.as_deref()
            .unwrap_or("No description");
        lines.push(Line::from(format!("  {}: {}", idx + 1, desc)));
    }
    
    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, area);
}

// All targets section (bottom)
fn render_all_targets(frame: &mut Frame, state: &SelectGeneratorState, area: Rect) {
    let mut lines = vec![
        Line::styled("Used by:", Style::default().add_modifier(Modifier::BOLD))
    ];
    
    // Combine and sort all targets with prefixes
    let mut all_targets: Vec<(String, &str)> = Vec::new();
    
    for target in &state.nixos_targets {
        all_targets.push((format!("nixos: {}", target), target.as_str()));
    }
    for target in &state.home_targets {
        all_targets.push((format!("home: {}", target), target.as_str()));
    }
    
    // Sort by the underlying name (already sorted by type due to construction)
    all_targets.sort_by(|a, b| a.1.cmp(b.1));
    
    // Display with truncation if needed
    const MAX_DISPLAY: usize = 10;
    let total = all_targets.len();
    
    for (idx, (display, _)) in all_targets.iter().take(MAX_DISPLAY).enumerate() {
        lines.push(Line::from(format!("  {}", display)));
    }
    
    if total > MAX_DISPLAY {
        let remaining = total - MAX_DISPLAY;
        lines.push(Line::styled(
            format!("  ... and {} more", remaining),
            Style::default().fg(Color::DarkGray)
        ));
    }
    
    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, area);
}
```

### Main Layout Integration

```rust
pub fn render_generator_selection(frame: &mut Frame, state: &SelectGeneratorState, area: Rect) {
    // Calculate dynamic heights based on content
    let desc_height = if state.artifact_description.is_some() { 2 } else { 1 };
    let prompts_height = if state.prompts.is_empty() { 0 } else { state.prompts.len() as u16 + 1 };
    
    // Calculate targets height (capped)
    let total_targets = state.nixos_targets.len() + state.home_targets.len();
    let targets_height = std::cmp::min(total_targets as u16 + 1, 12); // +1 for label, max 12
    
    // Main vertical layout
    let chunks = Layout::vertical([
        Constraint::Length(3),           // Title block with artifact name
        Constraint::Length(1),           // Type indicator
        Constraint::Length(desc_height), // Description
        Constraint::Length(prompts_height), // Prompts
        Constraint::Length(1),           // Separator
        Constraint::Min(5),              // Generator list (interactive)
        Constraint::Length(1),           // Separator
        Constraint::Length(targets_height), // All targets
        Constraint::Length(2),           // Help
    ])
    .margin(1)  // Margin within the block
    .split(area);
    
    // Render each section
    let title = format!("Select generator for artifact '{}'", state.artifact_name);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title);
    
    // Render sections inside the block area
    let inner_area = block.inner(area);
    let inner_chunks = Layout::vertical([
        Constraint::Length(1),           // Type indicator
        Constraint::Length(desc_height), // Description
        Constraint::Length(prompts_height), // Prompts
        Constraint::Length(1),           // Separator
        Constraint::Min(5),              // Generator list
        Constraint::Length(1),           // Separator
        Constraint::Length(targets_height), // All targets
        Constraint::Length(2),           // Help
    ]).split(inner_area);
    
    frame.render_widget(block, area);
    
    render_type_indicator(frame, state, inner_chunks[0]);
    render_description(frame, state, inner_chunks[1]);
    render_prompts_section(frame, state, inner_chunks[2]);
    render_separator(frame, inner_chunks[3]);
    render_generator_list(frame, state, inner_chunks[4]); // Existing list logic
    render_separator(frame, inner_chunks[5]);
    render_all_targets(frame, state, inner_chunks[6]);
    render_help(frame, state, inner_chunks[7]);
}
```

### Test Pattern (Snapshot Testing)

```rust
#[test]
fn test_enhanced_generator_selection_with_description() {
    let state = SelectGeneratorState {
        artifact_index: 0,
        artifact_name: "shared-cert".to_string(),
        artifact_description: Some("TLS certificate for internal services".to_string()),
        is_shared: true,
        prompts: vec![
            PromptEntry {
                name: "domain".to_string(),
                description: Some("Certificate domain (e.g., *.internal.example.com)".to_string()),
            },
            PromptEntry {
                name: "validity_days".to_string(),
                description: Some("Certificate validity in days".to_string()),
            },
        ],
        nixos_targets: vec!["server-1".to_string(), "server-2".to_string()],
        home_targets: vec!["alice@laptop".to_string()],
        generators: vec![GeneratorInfo {
            path: "/nix/store/abc123/generator.sh".to_string(),
            sources: vec![
                GeneratorSource {
                    target: "server-1".to_string(),
                    target_type: ConfigTargetType::Nixos,
                },
            ],
        }],
        selected_index: 0,
    };
    
    let backend = TestBackend::new(80, 25);
    let mut terminal = Terminal::new(backend).unwrap();
    
    terminal
        .draw(|f| render_generator_selection(f, &state, f.area()))
        .unwrap();
    
    let result = ViewTestResult {
        state: GeneratorSelectionSnapshot::from_state(&state),
        rendered: terminal.backend().to_string(),
    };
    assert_snapshot!(result.to_string());
}
```

---

## State of the Art

| Old Approach                               | Current Approach               | When Changed | Impact                                        |
| ------------------------------------------ | ------------------------------ | ------------ | --------------------------------------------- |
| Custom ANSI sequences                      | ratatui widgets                | Phase 1-2    | Standardized TUI, easier testing              |
| Manual layout math                         | ratatui Layout constraints     | Phase 1-2    | Responsive to terminal resize                 |
| Integration tests only                     | Unit tests + snapshot tests    | Phase 6-7    | Fast feedback, visual regression catching     |
| No visual distinction for shared artifacts | `[S]` marker and target counts | Phase 10     | Users can identify shared vs single artifacts |

---

## Open Questions

1. **Artifact Description Field**
   - What we know: Not currently in `ArtifactDef` or `SharedArtifactInfo`
   - What's unclear: Should it be added to the Nix module options first, or just
     Rust models?
   - Recommendation: Add to `modules/store.nix` artifact options, then propagate
     to Rust

2. **Long Description Handling**
   - What we know: User says "long prompt descriptions show full text (no
     truncation)"
   - What's unclear: What about very long artifact descriptions (paragraphs)?
   - Recommendation: Implement soft wrapping with
     `Paragraph::wrap(Wrap { trim: false })`

3. **Minimum Terminal Size**
   - What we know: Current tests use 70x15, 70x20, 80x25
   - What's unclear: What's the minimum size we should support for the enhanced
     dialog?
   - Recommendation: Maintain support down to 70x20, test edge cases at 60x15

---

## Sources

### Primary (HIGH confidence)

- `pkgs/artifacts/src/tui/views/generator_selection.rs` - Current dialog
  implementation
- `pkgs/artifacts/src/app/model.rs` - State structures (SelectGeneratorState,
  PromptEntry)
- `pkgs/artifacts/src/config/make.rs` - Config structures (ArtifactDef,
  SharedArtifactInfo, PromptDef)
- `pkgs/artifacts/src/tui/views/prompt.rs` - Reference for prompt description
  display pattern
- `pkgs/artifacts/tests/tui/view_tests.rs` - Snapshot testing patterns

### Secondary (MEDIUM confidence)

- ratatui 0.29 documentation patterns (from code usage)
- Existing snapshots in `pkgs/artifacts/tests/tui/snapshots/` - Visual reference
  for current output

### Tertiary (LOW confidence)

- None required — all patterns verifiable from codebase

---

## Metadata

**Confidence breakdown:**

- Standard stack: HIGH - ratatui 0.29 is locked, code patterns are established
- Architecture patterns: HIGH - Based on direct code inspection
- Pitfalls: HIGH - Derived from existing implementation constraints
- Data model changes: MEDIUM - Requires adding description field
  (straightforward but needs verification)

**Research date:** 2026-02-18\
**Valid until:** 2026-03-18 (30 days — stable codebase)

---

## Implementation Checklist for Planner

Based on research, the implementation tasks should include:

1. **Data Model Updates**
   - Add `description: Option<String>` to `ArtifactDef` in `src/config/make.rs`
   - Add `description` to `SharedArtifactInfo` in `src/config/make.rs`
   - Extend `SelectGeneratorState` in `src/app/model.rs` with new fields
   - Update `MakeConfiguration::get_shared_artifacts()` to populate description

2. **Nix Module Updates** (if description field is to be user-configurable)
   - Add `description` option to `modules/store.nix` artifact definition

3. **View Implementation**
   - Refactor `render_generator_selection()` in
     `src/tui/views/generator_selection.rs`
   - Add helper functions: `render_type_indicator()`, `render_description()`,
     `render_prompts_section()`, `render_all_targets()`
   - Implement path truncation utility
   - Implement target sorting (alphabetical)
   - Implement "+N more" truncation for >10 targets

4. **State Construction Updates**
   - Update `SelectGeneratorState` construction in `src/tui/model_builder.rs` or
     relevant location

5. **Testing**
   - Add snapshot tests for enhanced dialog with description
   - Add snapshot tests with many prompts
   - Add snapshot tests with >10 targets (verify "+N more")
   - Add snapshot tests with mixed NixOS/home-manager targets
   - Run `cargo insta review` and verify all snapshots

6. **Edge Cases**
   - No description provided (fallback text)
   - No prompts (skip prompts section)
   - Single target vs many targets
   - Very long descriptions (wrapping)
   - Very long generator paths (truncation)
