# Phase 2: Single Artifacts - Context

**Gathered:** 2026-02-13 **Status:** Ready for planning

<domain>
## Phase Boundary

Implement all single artifact effects (CheckSerialization, RunGenerator,
Serialize) with full script execution in the background. User can select a
single artifact and trigger generation. TUI remains responsive during script
execution (can navigate, scroll). CheckSerialization runs in background and
correctly determines if artifact needs generation. Generator script executes in
bubblewrap container without blocking UI. Serialize script completes and updates
artifact status in the list.

</domain>

<decisions>
## Implementation Decisions

### Generation Initiation

- Press `Enter` on selected artifact to trigger generation
- Confirmation prompt appears only for regeneration (not first-time generation)
- `a` key triggers "generate all" with confirmation dialog
- "Generate all" only generates artifacts that need generation, skips up-to-date
  ones
- If artifact is up-to-date and user triggers generation → prompt "Regenerate
  and override old artifact? y/n"
- Visual feedback: Status symbol changes to "generating" state in list + log
  panel shows "Generating..." when artifact selected

### Status Visibility

- Show current effect step: "CheckSerialization...", "Running generator...",
  "Serializing..."
- Script output (stdout/stderr) shown only after completion, not streamed live
- List updates immediately with status symbol while generating
- Full navigation allowed — user can scroll, select, and trigger other artifacts
  while one generates

### Error Presentation

- Errors appear in log/detail panel AND in artifact's state symbol (both
  locations visible)
- Show full stdout + stderr output for debugging purposes
- Failed artifacts show "Failed" status with retry option — user can regenerate
  to retry
- Failed artifacts have distinct color/symbol (e.g., red X or ⚠️ warning symbol)

### Cancel/Abort Behavior

- Can quit TUI with confirmation: "Effects are running, quit anyway? y/n"
- On quit: Cancel immediately, except serialization effects which must complete
  to avoid broken serialization state
- Can cancel individual artifacts with `c` or `Escape`, but running
  serialization effects continue to completion
- Duplicate generation requests on same artifact silently ignored (don't start
  duplicate)

### Claude's Discretion

- Exact symbols/colors for status states (generating, failed, success)
- Implementation of progress indicator animation (spinner style)
- Log panel scrolling behavior and capacity limits
- Confirmation dialog styling and key bindings
  </decisions>

<specifics>
## Specific Ideas

- Status symbols should be intuitive: generating = spinner/⟳, failed = red X/⚠️,
  success = ✓, needs generation = !
- Error output should be scrollable if it exceeds panel height
- Consider showing effect step name in the status column while generating
  </specifics>

<deferred>
## Deferred Ideas

- None — discussion stayed within phase scope
  </deferred>

---

_Phase: 02-single-artifacts_ _Context gathered: 2026-02-13_
