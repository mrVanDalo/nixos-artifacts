# Phase 14: Regeneration Confirmation Dialog - Context

**Gathered:** 2026-02-19 **Status:** Ready for planning

<domain>
## Phase Boundary

Add a confirmation dialog before regenerating existing artifacts to prevent
accidental overwrites. Users must explicitly confirm before overwriting, with
clear warnings and safe defaults. Covers both single artifacts and shared
artifacts with keyboard navigation support.

</domain>

<decisions>
## Implementation Decisions

### Dialog trigger conditions

- Trigger after `check_serialization` indicates artifact needs regeneration (not
  before)
- Only artifacts that already exist in backend storage trigger confirmation
- One dialog per artifact (not batch confirmation for multiple artifacts)
- Button selection only (no typed confirmation required)
- Default selection should be "not to regenerate" (safe choice)
- Only user-selected artifacts trigger dialog (not dependencies)

### Dialog UI design

- Buttons arranged side by side (left, right), not stacked vertically
- Warning text should warn about overwriting existing data
- Display artifact name and which machine(s) will be affected
- Centered modal dialog style
- Do NOT show reason for regeneration (not needed since user triggered it)

### Selection and navigation

- Keyboard navigation: both arrow keys (left/right) and Tab key
- Default selected button: Leave (the safe choice)
- Confirm selection with Enter or Space key
- Escape key behaves same as selecting Leave

### Status text behavior

- Status shown in dialog header
- Change from "Generating" to "Regenerating" immediately when dialog opens
- Different text for new vs existing artifacts
- Full sentence format: "Regenerating artifact: {name}"

### Shared artifacts handling

- Single dialog for shared artifacts (not per-machine)
- Show all affected machines in the dialog

### Claude's Discretion

- Exact visual styling (colors, borders, spacing)
- Specific warning message wording
- Button label text (beyond "Leave" and "Regenerate")
- Visual indicator for selected button

</decisions>

<specifics>
## Specific Ideas

- "Side by side (left, right) not on top each other" for button layout
- "Button label should be clear, default should be not to regenerate"
- "Full sentence like 'Regenerating artifact: ssh-key'" for status text
- "Show all affected machines" for shared artifact context

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

_Phase: 14-regeneration-confirmation_ _Context gathered: 2026-02-19_
