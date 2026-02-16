# State: v2.0 Robustness

**Project:** NixOS Artifacts Store — v2.0 Robustness\  
**Current Milestone:** v2.0 🔄 EXECUTING\
**Status:** Phase 6 In Progress — Edge case tests complete (06-04)
**Last Updated:** 2026-02-16 (completed 06-04)

---

## Project Reference

See: [.planning/PROJECT.md](./PROJECT.md) (updated 2026-02-16)

**Core Value:** The TUI must never freeze during long-running operations — all effect execution runs in a background job while the TUI remains interactive.

**Current Focus:** Defining v2.0 requirements for end-to-end tests, code quality, and logging improvements

---

## Current Position

| Aspect          | Status                      |
| --------------- | --------------------------- |
| Milestone       | v2.0 🔄 EXECUTING |
| Phase           | 06-integration-testing |
| Plan            | 06-04 ✅ COMPLETE |
| Requirements    | In Progress |
| Tests           | In Progress |
| Previous        | v1.0 ✅ SHIPPED (2026-02-15) |

### Progress Bar

```
[████████████░░░░░░░░] 40% complete — Phase 06 integration testing complete (06-01, 06-02, 06-03, 06-04 complete)
```

---

## Accumulated Context

### Decisions from v1.0

All v1.0 decisions preserved in PROJECT.md Validated section.

### New Decisions

- **Test approach:** Programmatic headless tests that invoke CLI and verify backend storage
- **Logging strategy:** Opt-in via `--log-output <file>` argument; no logging when not specified
- **Refactoring goal:** Flatten call chains, eliminate abbreviations, improve readability
- **State machine testing:** Dual assertion strategy - verify both command variants AND final model state
- **Test isolation:** Use #[serial] for async tests to prevent shared state conflicts
- **Async testing:** MockEventSource enables deterministic event-driven testing of async runtimes
- **CLI testing:** insta-cmd snapshots for CLI output verification (help, version, flags, error handling)
- **TUI integration tests:** Use sync run() intentionally when no real effects needed
- **E2E test results:** Store file contents instead of paths to handle temp directory cleanup
- **Headless API results:** Content-based storage (`generated_file_contents: BTreeMap<String, String>`) instead of path-based
- **Backend storage paths:** Test backend uses `{storage}/machines/{machine}/{artifact}/` structure
- **RAII cleanup:** Use CleanupGuard pattern for automatic environment variable cleanup in tests
- **Shared artifact testing:** Use headless `generate_single_artifact` for shared artifacts; stored per-machine in tests
- **Test documentation:** Document all test requirements in test file headers for CI visibility
- **Edge case testing:** Use existing error scenarios for realistic failure mode testing
- **Error message validation:** Focus on presence of key information, not exact wording

### Technical Debt to Address

**From v1.0:**
- End-to-end tests don't verify actual artifact creation in backend storage (addressed - TEST-03 and TEST-04 now verified)
- Some functions have deep call chains (f(g(h(k(...)))))
- Abbreviated variable names reduce readability
- Debug logging always writes to hardcoded path

### Completed

**06-01:**
- E2E test verification helpers (4 functions)
- Fixed temp directory cleanup in headless API
- TEST-01 and TEST-02 requirements documented
- All 5 e2e tests passing

**06-02:**
- Backend storage verification tests (5 tests)
- TEST-03: Verify artifact exists at backend location
- TEST-04: Verify artifact content matches expected format
- Edge case tests for multiple files, persistence, no-prompts scenarios

**06-03:**
- Shared artifact tests (5 tests in shared_artifact.rs)
- TEST-05: Tests cover both single-machine and shared artifacts
- TEST-06: Tests run in CI with meaningful failure messages
- All 6 TEST requirements marked complete in REQUIREMENTS.md
- 15 total e2e tests passing (6 mod.rs + 5 backend_verify.rs + 4 shared_artifact.rs)

**06-04:**
- Edge case tests (15 tests in edge_cases.rs)
- Error scenario tests: missing config, invalid backend, generator failure
- Serialization failure tests with proper error handling
- Artifact name validation: empty names, special characters
- Error message validation: context, actionability, no internal details
- 30 total e2e tests passing (previous 15 + 15 new edge case tests)

**06-05:**
- DiagnosticInfo struct with comprehensive diagnostic capture (headless.rs)
- generate_single_artifact_with_diagnostics() function for test debugging
- diagnostics.rs test module with 6 diagnostic tests
- Auto-dump on failure to /tmp/artifacts_test_failures/ with timestamps
- TESTING.md with comprehensive troubleshooting documentation
- Updated e2e_single_artifact_is_created to use diagnostic capture
- 36 total e2e tests passing (30 + 6 new diagnostic tests)

---

## TODOs

- [x] Define v2.0 requirements
- [ ] Create v2.0 roadmap
- [x] Phase 1: Integration tests for artifact creation (COMPLETE - all TEST requirements satisfied)
- [ ] Phase 2: Code quality refactoring
- [ ] Phase 3: Smart debug logging

### New Decisions

- **Diagnostic capture:** Always capture full context (config, env, temp files) on test failure
- **Security:** Redact sensitive values (prompts, secrets) rather than capture and filter
- **Human-readable:** Use section headers and visual separators, not debug formatting
- **Test documentation:** Document diagnostic system in TESTING.md for developers

### Blockers

None.

---

## Session Continuity

### Last Session

**Date:** 2026-02-16  
**Activity:** Completed 06-04 integration testing - edge case and error scenario tests  
**Summary:** Created edge_cases.rs with 15 comprehensive edge case tests. Tests cover missing config, invalid backend, generator failure, serialization failure, empty names, special characters. Error message validation tests verify context, actionability, and absence of internal details. All 30 e2e tests now passing. Duration: 15 min.

---

## Quick Links

- [PROJECT.md](./PROJECT.md) — Core value and constraints
- [REQUIREMENTS.md](./REQUIREMENTS.md) — v2.0 requirements (being defined)
- [ROADMAP.md](./ROADMAP.md) — Phase structure

