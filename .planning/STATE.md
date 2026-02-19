# State: v4.0 Regeneration Safety

**Project:** NixOS Artifacts Store — v4.0 Regeneration Safety
**Current Milestone:** v4.0 🚧 IN PROGRESS
**Status:** Phase 16 complete, ready for Phase 17
**Last Updated:** 2026-02-20

---

## Project Reference

See: [.planning/PROJECT.md](./PROJECT.md) (updated 2026-02-18)

**Core Value:** The TUI must never freeze during long-running operations — all
effect execution runs in a background job while the TUI remains interactive.

**Current Focus:** Phase 16 — Backend Developer Documentation

---

## Current Position

| Aspect       | Status                       |
| ------------ | ---------------------------- |
| Milestone    | v4.0 🚧 IN PROGRESS          |
| Phase        | 16 of 17 (complete)          |
| Plans        | 3 of 3 complete              |
| Requirements | Phase 16 requirements complete   |
| Last Activity | Completed Plan 16-03 (documentation navigation integration) |

### Progress Bar

```
[████████████████████████████████] 100% — Phase 16 complete: backend developer documentation with Antora guides, standalone BACKEND_GUIDE.md, and navigation integration
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

### Key Decisions from Phase 15

- HashSet<LogStep> for expanded_sections - O(1) toggle operations
- All sections expanded by default for immediate visibility
- Keyboard shortcuts: 'e' expand all, 'c' collapse all, Space toggle, Tab focus next
- Separate focused_section field for keyboard navigation distinct from expansion state
- ChronologicalLogState follows existing {Name}State naming pattern
- "Use 'l' key for log view (mnemonic for logs) - consistent with other single-letter shortcuts"
- ChronologicalLogState::new() constructor takes artifact_index and artifact_name for clean state creation
- Integration tests: add match arm for new Screen variants to prevent compilation errors

### Key Decisions from Phase 16 Plan 01

- Use partial includes for lifecycle diagram and quickstart to enable reuse
- Create comprehensive 600+ line guide rather than brief reference
- Include copy-paste templates for all 4 backend scripts
- Add migration notes from agenix-rekey and sops-nix

### Key Decisions from Phase 16 Plan 02

- Use Markdown instead of AsciiDoc for standalone BACKEND_GUIDE.md for universal compatibility
- Include complete working examples, not just snippets
- Create comprehensive environment variable reference tables
- Add troubleshooting section for common backend issues
- Design file to be copy-paste ready to other repositories

### Key Decisions from Phase 16 Plan 03

- Navigation placement: Backend Developer Guide follows brief reference and precedes usage guide
- Added See Also sections for better cross-linking between documentation pages
- Created dedicated Backend Development section in index.adoc for discoverability

### Technical Debt

**From v1.0-v3.0 (all addressed):**

- ✅ End-to-end tests verify actual artifact creation in backend storage
- ✅ Functions have flattened call chains
- ✅ No abbreviated variable names
- ✅ No hardcoded debug logging paths
- ✅ Shared artifact status transitions correctly
- ✅ Generator dialog shows rich context

**From Phase 16:**

- None - all documentation requirements met

### Roadmap Evolution

- Phase 15 added: Chronological Log View with Expandable Sections
- Phase 16 added: Backend Developer Documentation with Antora docs
- Phase 17 added: Model-based testing with full state capture

---

## Quick Links

- [PROJECT.md](./PROJECT.md) — Core value and requirements
- [Milestones](./milestones/) — Archived milestones
- [MILESTONES.md](./MILESTONES.md) — Milestone history
- [ROADMAP.md](./ROADMAP.md) — Current roadmap (to be created)
- [Phase 16 Plan 01 Summary](./phases/16-backend-dev-docs/16-01-SUMMARY.md) — Backend developer Antora docs
- [Phase 16 Plan 02 Summary](./phases/16-backend-dev-docs/16-02-SUMMARY.md) — Standalone BACKEND_GUIDE.md
- [Phase 16 Plan 03 Summary](./phases/16-backend-dev-docs/16-03-SUMMARY.md) — Documentation navigation integration

---

_Updated: 2026-02-20 — Phase 16 complete: backend developer documentation with Antora guides (lifecycle diagram, quickstart templates, navigation integration) and standalone BACKEND_GUIDE.md (733 lines, copy-paste ready)_
