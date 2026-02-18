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

**Current Focus:** Phase 14 — Regeneration Confirmation Dialog

---

## Current Position

| Aspect       | Status                       |
| ------------ | ---------------------------- |
| Milestone    | v4.0 🚧 IN PROGRESS          |
| Phase        | 14 of 14 (ready to plan)     |
| Plans        | 4 plans defined              |
| Requirements | 7/7 mapped to Phase 14       |
| Last Activity | Roadmap created             |

### Progress Bar

```
[░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░] 0% — Roadmap created, ready to plan Phase 14
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

---

## Quick Links

- [PROJECT.md](./PROJECT.md) — Core value and requirements
- [Milestones](./milestones/) — Archived milestones
- [MILESTONES.md](./MILESTONES.md) — Milestone history
- [ROADMAP.md](./ROADMAP.md) — Current roadmap (to be created)

---

_Updated: 2026-02-18 — v4.0 milestone started_
