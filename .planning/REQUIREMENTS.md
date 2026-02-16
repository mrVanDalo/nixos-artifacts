# Requirements: NixOS Artifacts Store — v2.0 Robustness

**Defined:** 2026-02-16 **Core Value:** The TUI must never freeze during long-running operations — all effect execution runs in a background job while the TUI remains interactive.

---

## v1 Requirements (v2.0 Scope)

### Testing — End-to-End Verification

- [x] **TEST-01**: Test can programmatically invoke artifact generation without TUI
- [x] **TEST-02**: Test creates a single artifact with simple configuration
- [x] **TEST-03**: Test verifies generated artifact exists at expected backend location
- [x] **TEST-04**: Test verifies artifact content matches expected format
- [x] **TEST-05**: Test covers both single-machine and shared artifacts
- [x] **TEST-06**: Test runs as part of CI/test suite and fails if artifacts not created

### Code Quality — Readability & Structure

- [ ] **QUAL-01**: No function chains deeper than 2 levels (f(g(x)) allowed, f(g(h(x))) not allowed)
- [ ] **QUAL-02**: Functions return results that are passed to next function, not nested calls
- [ ] **QUAL-03**: All function names are descriptive and unabbreviated (min 3 words preferred)
- [ ] **QUAL-04**: All variable names are descriptive and unabbreviated (no `cfg`, `hdl`, `ctx`)
- [ ] **QUAL-05**: Functions are under 50 lines (except match-heavy update functions)
- [ ] **QUAL-06**: Each function has single, clear responsibility
- [ ] **QUAL-07**: Refactoring limited to `pkgs/artifacts/src/` directory

### Logging — Opt-in Debug Output

- [ ] **LOG-01**: CLI accepts `--log-output <file>` argument
- [ ] **LOG-02**: When `--log-output` is provided, comprehensive debug logs written to specified file
- [ ] **LOG-03**: When `--log-output` is not provided, no debug logging occurs
- [ ] **LOG-04**: Debug logs include: timestamps, effect execution, channel messages, backend calls
- [ ] **LOG-05**: Log file path can be absolute or relative
- [ ] **LOG-06**: Existing `/tmp/artifacts_debug.log` hardcoded path removed

---

## v2 Requirements (Deferred)

### Future Code Quality

- **QUAL-08**: Apply naming conventions to Nix modules (not just Rust code)
- **QUAL-09**: Add linting rules to enforce naming conventions
- **QUAL-10**: Documentation for all public functions

### Future Testing

- **TEST-07**: Fuzz testing for artifact name formats
- **TEST-08**: Performance benchmarks for generation time
- **TEST-09**: Stress tests with 100+ artifacts

### Future Logging

- **LOG-07**: Structured logging (JSON format option)
- **LOG-08**: Log level filtering (trace, debug, info)
- **LOG-09**: Console output in addition to file logging

---

## Out of Scope

| Feature | Reason |
|---------|--------|
| Progress bars | Not needed for v2.0; atomic operations are fast enough |
| Effect cancellation | Complex UX, defer to v3.0 |
| Concurrent execution | Sequential processing is correct and simple |
| Web-based UI | CLI + TUI sufficient for v2.0 |
| Metrics/telemetry | Out of scope for robustness milestone |

---

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| TEST-01 | Phase 6 | ✅ Complete |
| TEST-02 | Phase 6 | ✅ Complete |
| TEST-03 | Phase 6 | ✅ Complete |
| TEST-04 | Phase 6 | ✅ Complete |
| TEST-05 | Phase 6 | ✅ Complete |
| TEST-06 | Phase 6 | ✅ Complete |
| QUAL-01 | Phase 7 | Pending |
| QUAL-02 | Phase 7 | Pending |
| QUAL-03 | Phase 7 | Pending |
| QUAL-04 | Phase 7 | Pending |
| QUAL-05 | Phase 7 | Pending |
| QUAL-06 | Phase 7 | Pending |
| QUAL-07 | Phase 7 | Pending |
| LOG-01 | Phase 8 | Pending |
| LOG-02 | Phase 8 | Pending |
| LOG-03 | Phase 8 | Pending |
| LOG-04 | Phase 8 | Pending |
| LOG-05 | Phase 8 | Pending |
| LOG-06 | Phase 8 | Pending |

**Coverage:**

- v1 requirements: 18 total
- Mapped to phases: 18
- Unmapped: 0 ✓

---

_Requirements defined: 2026-02-16_ _Last updated: 2026-02-16 after 06-03 - all TEST requirements completed_
