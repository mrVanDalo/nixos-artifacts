# Phase 5: Validation — Testing - Context

**Gathered:** 2026-02-14 **Status:** Ready for planning

<domain>
## Phase Boundary

Update all tests to work with the new async channel-based architecture. Includes: runtime tests for async channel communication, effect handler tests using channel-based mocks, integration tests passing end-to-end, and view tests remaining unchanged.

</domain>

<decisions>
## Implementation Decisions

### Mock Strategy for Background Job

- **Channel-level mocks** for unit tests — mock the tokio mpsc channel (sender/receiver), not handler-level abstractions
- **No mocks for integration tests** — use real background job with actual script execution
- **Controlled async delays** — use `tokio::time::sleep` for realistic async timing in tests
- **Full state machine simulation** — mock simulates complete lifecycle: Pending → Running → Success/Failed
- **Dual assertion strategy** — verify both: (1) commands sent to mock match expected `EffectCommand` variants, AND (2) final Model state updated correctly

### Test Coverage Targets

- **80% minimum coverage** for async channel components (standard industry threshold)
- **Critical error scenarios** — must test: channel disconnect and timeout (the two critical failure modes)
- **Background job branches** — cover main loop (`tokio::select!` branches) + error handling paths
- **Comprehensive shutdown tests** — graceful shutdown, forced shutdown, and timeout scenarios

### Async Test Patterns

- **Sequential execution** — use `#[serial]` attribute to avoid shared state conflicts in async tests
- **Dedicated test directory** — place async tests in `tests/async/` directory, separate from existing tests
- **Mock time for timing tests** — use `tokio::time::pause()` and manual time advancement for timeout/delay tests (not real time)
- **Test declaration** — Claude's discretion: choose most robust and readable pattern

### Integration Test Updates

- **Existing test updates** — Claude's discretion: determine what needs updating (insta-cmd tests)
- **No new async-specific integration tests** — unit tests cover async logic; integration tests verify CLI interface
- **Fast test scripts** — use mock generators/check scripts that exit immediately for TUI flow tests
- **No dedicated CI configuration** — run all tests together as currently configured
- **Insta snapshot focus** — prioritize snapshot testing over assertion chains for readability and maintainability

### Claude's Discretion

- Exact test declaration style for async functions
- Which existing integration tests need updating
- Specific organization within `tests/async/` directory

</decisions>

<specifics>
## Specific Ideas

- "I want to make sure that integration tests should still focus on insta snapshots. Try to avoid assertion chains when possible."
- Use `tokio::time::pause()` for reliable timeout testing without real delays
- Sequential test execution prevents flaky async test failures
- Channel-level mocking keeps tests close to actual implementation

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

_Phase: 05-validation_ _Context gathered: 2026-02-14_
