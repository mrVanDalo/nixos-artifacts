# Phase 17: Model-based testing with full state capture - Context

**Gathered:** 2026-02-19 **Status:** Ready for planning

<domain>
## Phase Boundary

Improve testing infrastructure to better document and verify the Elm Architecture pattern by separating Model transformation tests from View rendering tests. Current view tests only render a small portion of the model, but integration tests capture the full model state changes. This phase brings that same full state capture to view tests, enabling developers to trace how inputs affect the model and how model states render to different views.

</domain>

<decisions>
## Implementation Decisions

### Test Organization Structure

- Keep existing file structure (don't change from current layout)
- Use different modules within test files (e.g., `mod model_tests`, `mod view_tests`)
- Same file can contain both model and view tests, clearly separated by modules
- Integration tests already demonstrate the pattern to follow in `tests/tui/integration_tests.rs`

### State Capture Format

- Use the same format as current integration tests (`tests/tui/integration_tests.rs`)
- Custom structs like `TestResult`, `ModelState`, `ArtifactState` that derive Debug
- Full model state captured via `assert_debug_snapshot!` macro from insta crate
- Derive(Debug) trait provides automatic field capture with all values
- No custom Debug implementations needed - use derive macro

### Test Flow Documentation

- Use dual assertions: capture both full Model state AND rendered View
- Event vector only for input sequence (e.g., `vec![enter(), type_text("secret")]`)
- No commented event descriptions needed - the event names are self-documenting
- Test documents the chain: inputs → Model transformation → View rendering

### Assertion Patterns

- Single comprehensive assertion per test that captures everything
- Test captures: events applied, Model before, Model after, serialized artifacts
- Use `assert_debug_snapshot!` for structured state snapshots
- All assertions use insta snapshot testing for consistent formatting

### Claude's Discretion

- Exact snapshot file naming conventions
- ModelState struct field selection (which fields to include)
- Helper function design for test setup
- Module visibility and exports

</decisions>

<specifics>
## Specific Ideas

- Follow the pattern already established in `tests/tui/integration_tests.rs` which has `ModelState::from_model()` and captures full state
- Integration tests show the desired behavior: they capture `before` and `after` Model state using Debug snapshots
- Want view tests to have same visibility into model changes, not just view snapshots
- Key insight: view tests currently only show terminal buffer, but want to see what Model state produced that view

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

_Phase: 17-model-based-testing-with-full-state-capture_ _Context gathered: 2026-02-19_
