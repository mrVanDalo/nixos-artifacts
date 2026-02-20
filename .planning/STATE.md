# State: v4.0 Regeneration Safety

**Project:** NixOS Artifacts Store — v4.0 Regeneration Safety
**Current Milestone:** v4.0 🚧 IN PROGRESS
**Status:** Phase 17 complete, Plan 2 complete
**Last Updated:** 2026-02-20

---

## Project Reference

See: [.planning/PROJECT.md](./PROJECT.md) (updated 2026-02-18)

**Core Value:** The TUI must never freeze during long-running operations — all
effect execution runs in a background job while the TUI remains interactive.

**Current Focus:** Phase 17 — Model-based Testing with Full State Capture

---

## Current Position

| Aspect       | Status                       |
| ------------ | ---------------------------- |
| Milestone    | v4.0 🚧 IN PROGRESS          |
| Phase        | 17 of 17 (in progress)       |
| Plans        | 2 of 2 complete              |
| Requirements | Model-based testing infrastructure established |
| Last Activity | Completed Plan 17-02 (view tests with Model state capture) |

### Progress Bar

```
[████████████████████████████████] 100% — Phase 17 complete: Model-based testing with full state capture - shared ModelState infrastructure and dual assertion pattern in view tests
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

### Key Decisions from Phase 17 Plan 01

- ModelState struct uses #[derive(Debug)] for automatic field capture in snapshots
- Shared module pattern in tests/tui/ enables reuse across integration and view tests
- Included warnings_count field for comprehensive state representation
- normalize_status centralized in shared module for environment-independent snapshots

### Key Decisions from Phase 17 Plan 02

- Option<ModelState> pattern enables backward compatibility: Some(ModelState) for Model-based tests, None for screen-state tests
- Dual assertion pattern: capture both view-specific state AND full Model state in same test
- Three-section snapshot format: State, Model (optional), Rendered - documents Elm Architecture chain
- Artifact list tests capture complete Model state; prompt/progress/generator tests use existing comprehensive snapshot structs

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

**From Phase 17:**

- None - shared ModelState infrastructure complete

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
- [Phase 17 Plan 01 Summary](./phases/17-model-based-testing-with-full-state-capture/17-01-SUMMARY.md) — Shared ModelState for test state capture
- [Phase 17 Plan 02 Summary](./phases/17-model-based-testing-with-full-state-capture/17-02-SUMMARY.md) — View tests with Model state capture
- [Phase 17 Plan 02 Summary](./phases/17-model-based-testing-with-full-state-capture/17-02-SUMMARY.md) — View tests with Model state capture

---

_Updated: 2026-02-20 — Phase 17 complete: shared ModelState infrastructure with Debug trait pattern, view tests updated with dual assertion pattern capturing both view state and full Model state_
