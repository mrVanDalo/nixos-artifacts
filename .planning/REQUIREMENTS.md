# Requirements: v4.1 Code Quality & Documentation Cleanup

**Defined:** 2026-02-22  
**Core Value:** The TUI must never freeze during long-running operations — all effect execution runs in a background job while the TUI remains interactive.

## v1 Requirements

### Compiler & Linter Warnings

- [ ] **LINT-01**: Main code compiles with zero compiler warnings (`cargo build`)
- [ ] **LINT-02**: Main code passes clippy with zero warnings (`cargo clippy`)
- [ ] **LINT-03**: Tests compile with zero compiler warnings (`cargo test --no-run`)
- [ ] **LINT-04**: Tests pass clippy with zero warnings (`cargo clippy --tests`)
- [ ] **LINT-05**: All clippy lints enabled and addressed (pedantic, nursery where appropriate)

### Dead Code Elimination

- [ ] **DEAD-01**: No unused functions in main codebase
- [ ] **DEAD-02**: No unused variables (prefix with underscore if intentionally unused)
- [ ] **DEAD-03**: No unused imports
- [ ] **DEAD-04**: No unreachable code paths
- [ ] **DEAD-05**: No dead_code attributes without justification comments

### Unused File Cleanup

- [ ] **FILE-01**: All documentation files in `docs/` are referenced and rendered
- [ ] **FILE-02**: No empty files in repository (except intentionally kept placeholders)
- [ ] **FILE-03**: All `CLAUDE.md` files are actively used by AI workflows
- [ ] **FILE-04**: All `README.md` files are current and not orphaned
- [ ] **FILE-05**: Remove or consolidate documentation not part of build output

### Rust Documentation

- [ ] **DOC-01**: All public modules have module-level documentation (`//!`)
- [ ] **DOC-02**: All public functions have doc comments (`///`) with description and examples
- [ ] **DOC-03**: All public structs/enums have doc comments with field descriptions
- [ ] **DOC-04**: All trait implementations documented
- [ ] **DOC-05**: All complex functions have inline comments explaining logic
- [ ] **DOC-06**: `cargo doc` generates without warnings or broken links
- [ ] **DOC-07**: Documentation includes examples for public APIs
- [ ] **DOC-08**: All `# Panics`, `# Errors`, and `# Safety` sections documented

### Dependency Audit

- [ ] **DEPS-01**: All dependencies in Cargo.toml are used (no unused deps)
- [ ] **DEPS-02**: All features in Cargo.toml are used (no unused features)
- [ ] **DEPS-03**: No duplicate dependencies where avoidable

## v2 Requirements

Deferred to future milestone.

### Advanced Linting

- **ADV-01**: Enable additional clippy categories (restriction, cargo lints)
- **ADV-02**: Add cargo-deny for license and security auditing
- **ADV-03**: Add cargo-outdated for dependency freshness tracking

### Code Metrics

- **MET-01**: Function complexity analysis (cyclomatic complexity)
- **MET-02**: Code coverage reporting integration
- **MET-03**: Automated code quality gates in CI

## Out of Scope

| Feature | Reason |
|---------|--------|
| Major refactoring | This is cleanup, not redesign. Keep existing structure. |
| Performance optimization | Different milestone focus. Only remove dead code, don't optimize. |
| API redesign | Breaking changes allowed only for dead code removal, not API changes. |
| New features | Scope is cleanup only. |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| LINT-01 | Phase 18 | Pending |
| LINT-02 | Phase 18 | Pending |
| LINT-03 | Phase 18 | Pending |
| LINT-04 | Phase 18 | Pending |
| LINT-05 | Phase 18 | Pending |
| DEAD-01 | Phase 19 | Pending |
| DEAD-02 | Phase 19 | Pending |
| DEAD-03 | Phase 19 | Pending |
| DEAD-04 | Phase 19 | Pending |
| DEAD-05 | Phase 19 | Pending |
| FILE-01 | Phase 20 | Pending |
| FILE-02 | Phase 20 | Pending |
| FILE-03 | Phase 20 | Pending |
| FILE-04 | Phase 20 | Pending |
| FILE-05 | Phase 20 | Pending |
| DOC-01 | Phase 21 | Pending |
| DOC-02 | Phase 21 | Pending |
| DOC-03 | Phase 21 | Pending |
| DOC-04 | Phase 21 | Pending |
| DOC-05 | Phase 21 | Pending |
| DOC-06 | Phase 21 | Pending |
| DOC-07 | Phase 21 | Pending |
| DOC-08 | Phase 21 | Pending |
| DEPS-01 | Phase 22 | Pending |
| DEPS-02 | Phase 22 | Pending |
| DEPS-03 | Phase 22 | Pending |

**Coverage:**

- v1 requirements: 24 total
- Mapped to phases: 24/24 ✓
- Unmapped: 0 ✓

**Phase Summary:**

| Phase | Name | Requirement Count |
|-------|------|---------------------|
| Phase 18 | Fix Compiler & Clippy Warnings | 5 |
| Phase 19 | Dead Code Elimination | 5 |
| Phase 20 | Unused File Cleanup | 5 |
| Phase 21 | Rust Documentation | 8 |
| Phase 22 | Dependency Audit | 3 |
| **Total** | | **24** |

---

_Requirements defined: 2026-02-22_  
_Last updated: 2026-02-22 after roadmap creation_
