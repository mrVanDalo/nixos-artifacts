# Phase 13: Enhanced Generator Dialog - Context

**Gathered:** 2026-02-18 **Status:** Ready for planning

<domain>
## Phase Boundary

Improve the generator selection dialog in the TUI to display rich context about
artifacts. The dialog must show: artifact name, description if available, prompt
descriptions, shared vs per-machine status, and which machines/users use the
artifact. This phase focuses on presentation and layout — not adding new
capabilities.

</domain>

<decisions>
## Implementation Decisions

### Layout and information hierarchy

- Artifact type indicator (shared vs per-machine) appears at the very top
- Artifact name appears in the dialog title/header itself
- Artifact description appears above the generator list (if available)
- Prompt descriptions appear in a dedicated section with clear labels, displayed
  before the generator selection
- Machine/user info appears at the bottom of the dialog
- Sections are visually separated by line separators
- Machines/users are alphabetically sorted

### Visual presentation style

- Shared vs per-machine status indicated with text labels only (no color-coding)
- NixOS machines vs home-manager distinguished by type prefix: `nixos:` and
  `home:`
- Long generator paths (Nix store paths) are truncated with ellipsis
- Currently selected generator indicated with arrow `>` symbol
- Prompt descriptions displayed as labeled list items (e.g., "Prompt 1:
  Description")

### Content display

- Long prompt descriptions show full text (no truncation)
- If artifact has no description defined: show fallback text "No description
  provided"
- Generator context (machine/user info) formatted consistently showing full
  identifier (machine name for NixOS, user@host for home-manager)

### Machine/user listing format

- Each machine/user displayed on its own line (vertical list)
- If more than 10 machines: show first 10 with "+N more" indicator

### Claude's Discretion

- Exact spacing and padding between sections
- Whether to use bold/italic text for labels vs values
- Specific character limit for truncated paths
- Exact formatting of the "+N more" indicator text

</decisions>

<specifics>
## Specific Ideas

- No specific references or examples provided — open to standard terminal UI
  patterns

</specifics>

<deferred>
## Deferred Ideas

- None — discussion stayed within phase scope

</deferred>

---

_Phase: 13-enhanced-generator-dialog_ _Context gathered: 2026-02-18_
