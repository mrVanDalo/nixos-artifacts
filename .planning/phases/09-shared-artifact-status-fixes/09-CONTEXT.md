# Phase 9: Shared Artifact Status Fixes - Context

**Gathered:** 2026-02-18 **Status:** Ready for planning

<domain>
## Phase Boundary

Fix TUI status icons for shared artifacts to show correct aggregation state (needs-generation/up-to-date/failed) instead of generic "pending" status. Handle error case when shared artifact definitions have mismatched file names across machines.

</domain>

<decisions>
## Implementation Decisions

### Status Icon Representation

- Use same icons as single artifacts (not distinct visual treatment)
- Status is all-or-none: up-to-date, needs-generation, or failed
- No per-machine breakdown in icon/label
- Failed state (including script crashes) uses standard failed icon

### Aggregation Logic

- Status comes directly from `shared_check_serialization` script result
- No per-machine aggregation - the script handles all machines atomically
- Exit code determines status: success = up-to-date, specific code = needs-generation, failure = failed
- Status transitions: pending → final state after check completes, never returns to pending

### Error State (Mismatched File Definitions)

- Trigger: When shared artifact has different file definitions across machines
- Icon: Same as "failed" state
- Detail pane: Shows error message explaining the misconfiguration
- Behavior: Generation disabled for this artifact
- User action: If user tries to generate, show misconfiguration message
- Caching: Error state is cached (TUI doesn't re-trigger checks on refresh)

### Status Display in UI

- Shared badge: Keep current "shared" indicator (as implemented)
- List ordering: Mixed with single artifacts (not grouped separately)
- Selection: When selected, show generator selection dialog (unless in error state)
- Detail pane: Error messages appear in right-side detail pane when error-state artifact selected

### Generator Selection Behavior

- Skip dialog: When only one unique generator (by Nix store path) exists across all machines
- Show dialog: When multiple unique generators exist
- Error state override: Never show generator dialog when artifact is in error state

### Claude's Discretion

- Exact wording of error messages
- Visual styling of "shared" badge
- Detail pane layout and formatting
- How the misconfiguration message is displayed when user attempts generation

</decisions>

<specifics>
## Specific Ideas

- The shared artifact status must match the actual backend state after `shared_check_serialization` runs
- File name definitions must be identical across all machine references to a shared artifact
- Generator scripts can differ (that's why selection dialog exists), but file outputs must match

</specifics>

<deferred>
## Deferred Ideas

- None — discussion stayed within phase scope

</deferred>

---

_Phase: 09-shared-artifact-status-fixes_ _Context gathered: 2026-02-18_
