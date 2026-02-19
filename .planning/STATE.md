# State: v4.0 Regeneration Safety

**Project:** NixOS Artifacts Store — v4.0 Regeneration Safety
**Current Milestone:** v4.0 🚧 IN PROGRESS
**Status:** Roadmap created, ready to plan
**Last Updated:** 2026-02-18

---

## Project Reference

See: [.planning/PROJECT.md](./PROJECT.md) (updated 2026-02-18)

**Core Value:** The TUI must never freeze during long-running operations — all
effect execution runs in a background job while the TUI remains interactive.

**Current Focus:** Phase 15 — Chronological Log View with Expandable Sections

---

## Current Position

| Aspect       | Status                       |
| ------------ | ---------------------------- |
| Milestone    | v4.0 🚧 IN PROGRESS          |
| Phase        | 15 of 15 (in progress)       |
| Plans        | 3 of 3 complete              |
| Requirements | Phase 15 requirements active   |
| Last Activity | Completed Plan 15-02 (chronological log view navigation) |

### Progress Bar

```
[████████████████████████████████] 100% — Phase 15 complete: chronological log view with expandable sections and navigation
```

---

## Accumulated Context

### Decisions from v1.0, v2.0, v3.0

All decisions preserved in PROJECT.md Validated section.

### Key Decisions from v3.0

- Description field pattern: Option<String> with serde(default) for backward compatibility
- Shared artifact aggregation: Description from first artifact (consistent with prompts/files)
- Nix export pattern: Use builtins.mapAttrs to wrap artifacts with optional field handling
- Generator selection: Compare by Nix store path for true uniqueness
- Error handling: Pre-terminal config loading, panic handler with terminal restoration
- Script output: Real-time streaming via OutputLine messages

### Key Decisions from Phase 14

- exists flag defaults to false until check_serialization proves otherwise
- Check script convention: Scripts output "EXISTS" keyword to signal artifact already exists
- Leave button is default selection (safe choice) - prevents accidental regeneration
- Dialog only appears when exists=true AND status=NeedsGeneration
- Full sentence format: "{Verb} artifact: {name}" for clarity instead of "Generating: {name}"
- Verb determined by exists flag: Regenerating for existing, Generating for new
- Status text shown in list view: "Regenerating..." or "Generating..." during active generation
- Exists flag flows from entry → GeneratingState → progress view for consistent UX
- Comprehensive test suite: 26 test cases covering all dialog behaviors
- Test coverage: state transitions, keyboard navigation, visual snapshots, edge cases
- Visual regression testing with 4 insta snapshots for dialog appearance

### Key Decisions from Phase 15 Plan 02

- "Use 'l' key for log view (mnemonic for logs) - consistent with other single-letter shortcuts"
- "ChronologicalLogState::new() constructor takes artifact_index and artifact_name for clean state creation"
- "All sections expanded by default - user sees all logs immediately"
- "Integration tests: add match arm for new Screen variants to prevent compilation errors"

### Key Decisions from Phase 15 Plan 01

- HashSet<LogStep> for expanded_sections - O(1) toggle operations
- All sections expanded by default for immediate visibility
- Keyboard shortcuts: 'e' expand all, 'c' collapse all, Space toggle, Tab focus next
- Separate focused_section field for keyboard navigation distinct from expansion state
- ChronologicalLogState follows existing {Name}State naming pattern

### Technical Debt

**From v1.0-v3.0 (all addressed):**

- ✅ End-to-end tests verify actual artifact creation in backend storage
- ✅ Functions have flattened call chains
- ✅ No abbreviated variable names
- ✅ No hardcoded debug logging paths
- ✅ Shared artifact status transitions correctly
- ✅ Generator dialog shows rich context

### Roadmap Evolution

- Phase 15 added: Chronological Log View with Expandable Sections
- Phase 16 added: Backend Developer Documentation with Antora docs + BACKEND_GUIDE.md context file for AI assistants
- Phase 17 added: Model-based testing with full state capture

---

## Quick Links

- [PROJECT.md](./PROJECT.md) — Core value and requirements
- [Milestones](./milestones/) — Archived milestones
- [MILESTONES.md](./MILESTONES.md) — Milestone history
- [ROADMAP.md](./ROADMAP.md) — Current roadmap (to be created)

---

_Updated: 2026-02-19 — Phase 15 complete: chronological log view with expandable sections and navigation_
