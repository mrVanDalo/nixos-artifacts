# Requirements: v3.0 TUI Polish

**Defined:** 2026-02-18  
**Core Value:** The TUI must never freeze during long-running operations — all
effect execution runs in a background job while the TUI remains interactive.

## v1 Requirements (v3.0 TUI Polish)

Requirements for v3.0 TUI Polish milestone. Each maps to roadmap phases.

### UI/UX Fixes

- [ ] **UI-01**: Shared artifacts transition from pending to correct status after check completes (needs-generation/up-to-date)
- [ ] **UI-02**: Generator selection dialog skips when only one unique generator (same Nix store path)
- [ ] **UI-03**: TUI crashes display errors to stderr; all other output goes only to log file when --log-file given
- [ ] **UI-04**: Script output (stdout/stderr from check/generator/serialize) visible in TUI interface
- [ ] **UI-05**: Generator dialog shows machine/user/home-manager context, shared status, artifact name, prompt descriptions

### Status Display

- [ ] **STAT-01**: Status icons correctly reflect artifact state (pending, needs-generation, up-to-date, generating, done, failed)
- [ ] **STAT-02**: Shared artifact aggregation properly calculates combined status across machines

### Output Capture

- [ ] **OUT-01**: Script stdout captured and stored for TUI display
- [ ] **OUT-02**: Script stderr captured and stored for TUI display
- [ ] **OUT-03**: Output display updates in real-time during script execution
- [ ] **OUT-04**: Previous script output accessible in artifact detail view

### Error Handling

- [ ] **ERR-01**: TUI initialization failures print clear error to stderr before exit
- [ ] **ERR-02**: Terminal restoration failures print clear error to stderr
- [ ] **ERR-03**: All runtime errors visible in TUI, not stdout/stderr
- [ ] **ERR-04**: Panic handler prints to stderr and attempts terminal restoration

### Generator Selection

- [ ] **GEN-01**: Generators compared by Nix store path for uniqueness
- [ ] **GEN-02**: Single unique generator automatically selected without dialog
- [ ] **GEN-03**: Multiple unique generators show selection dialog with full context
- [ ] **GEN-04**: Dialog shows generator context: machine name, user name, home-manager vs nixos

### Enhanced Dialog

- [ ] **DIALOG-01**: Generator dialog displays artifact name
- [ ] **DIALOG-02**: Generator dialog displays artifact description if available
- [ ] **DIALOG-03**: Generator dialog shows all prompt descriptions for the artifact
- [ ] **DIALOG-04**: Generator dialog indicates if artifact is shared across machines
- [ ] **DIALOG-05**: Generator dialog shows which machines/users use this artifact

## v2 Requirements

Deferred to future release. Tracked but not in current roadmap.

### Progress Reporting

- **PROG-01**: Progress percentage shown during long-running effects
- **PROG-02**: Spinner or progress bar for active operations
- **PROG-03**: Estimated time remaining for known-duration operations

### Cancellation

- **CANC-01**: User can cancel in-flight effects via keybinding
- **CANC-02**: Cancellation propagates to background job
- **CANC-03**: Partial cleanup on cancellation

### Concurrent Execution

- **CONC-01**: Independent effects can execute concurrently
- **CONC-02**: Artifact dependencies respected in concurrent execution
- **CONC-03**: Resource limits for concurrent script execution

## Out of Scope

Explicitly excluded. Documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| Real-time log tailing | Complex terminal UI, defer to v3.1+ |
| Interactive prompt editing | Not requested, current prompts sufficient |
| Script input/interaction | Scripts must be fully automated |
| Performance profiling | Not a user-facing TUI concern |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase   | Status  |
|-------------|---------|---------|
| UI-01       | Phase 9 | Pending |
| UI-02       | Phase 10 | Pending |
| UI-03       | Phase 11 | Pending |
| UI-04       | Phase 12 | Pending |
| UI-05       | Phase 13 | Pending |
| STAT-01     | Phase 9 | Pending |
| STAT-02     | Phase 9 | Pending |
| OUT-01      | Phase 12 | Pending |
| OUT-02      | Phase 12 | Pending |
| OUT-03      | Phase 12 | Pending |
| OUT-04      | Phase 12 | Pending |
| ERR-01      | Phase 11 | Pending |
| ERR-02      | Phase 11 | Pending |
| ERR-03      | Phase 11 | Pending |
| ERR-04      | Phase 11 | Pending |
| GEN-01      | Phase 10 | Pending |
| GEN-02      | Phase 10 | Pending |
| GEN-03      | Phase 10 | Pending |
| GEN-04      | Phase 10 | Pending |
| DIALOG-01   | Phase 13 | Pending |
| DIALOG-02   | Phase 13 | Pending |
| DIALOG-03   | Phase 13 | Pending |
| DIALOG-04   | Phase 13 | Pending |
| DIALOG-05   | Phase 13 | Pending |

**Coverage:**

- v1 requirements: 20 total
- Mapped to phases: 20 ✓
- Unmapped: 0 ✓

---

_Requirements defined: 2026-02-18_  
_Updated: 2026-02-18 (roadmap complete)_
