# Roadmap: v4.1 Code Quality & Documentation Cleanup

**Milestone:** v4.1 Code Quality & Documentation Cleanup  
**Phases:** 5 (18-22)  
**Requirements:** 24 v1 requirements  
**Coverage:** 100% (24/24 mapped)  
**Depth:** Comprehensive  
**Defined:** 2026-02-22  

---

## Overview

This roadmap delivers a comprehensive cleanup of the NixOS Artifacts Store codebase. The focus is on achieving zero compiler warnings, eliminating dead code, cleaning up unused files, documenting all public APIs, and auditing dependencies. Each phase builds on the previous to create a clean, well-documented, and maintainable codebase.

The phases follow a logical order: first fix active code issues (linting), then remove what's not needed (dead code, unused files), then document what remains, and finally audit external dependencies.

---

## Phase 18: Fix Compiler & Clippy Warnings

**Goal:** Achieve zero warnings from both rustc and clippy across main code and tests

**Phase Number:** 18  
**Requirements:** LINT-01, LINT-02, LINT-03, LINT-04, LINT-05  
**Dependencies:** None (can start immediately)  
**Status:** ✅ COMPLETE (2026-02-22)

**Plans:** 5 plans (all complete)

### Success Criteria

1. **Main code compiles with zero warnings** — `cargo build` completes with no compiler warnings
2. **Main code passes clippy with zero warnings** — `cargo clippy` completes with no warnings at default level
3. **Tests compile with zero warnings** — `cargo test --no-run` completes with no compiler warnings
4. **Tests pass clippy with zero warnings** — `cargo clippy --tests` completes with no warnings
5. **Pedantic lints addressed** — Additional clippy lints from clippy::pedantic and clippy::nursery are reviewed and addressed where appropriate

---

## Phase 19: Dead Code Elimination

**Goal:** Remove all dead code including unused functions, variables, imports, and unreachable paths

**Phase Number:** 19  
**Requirements:** DEAD-01, DEAD-02, DEAD-03, DEAD-04, DEAD-05  
**Dependencies:** Phase 18 (fix warnings first to see actual dead code clearly)
**Status:** 🚧 IN PROGRESS (2026-02-22)

**Plans:** 1 plan

- [ ] 19-01 — Identify and remove all dead code from the Rust codebase

### Success Criteria

1. **No unused functions in main codebase** — All functions are called or marked with `#[allow(dead_code)]` and justification comment
2. **No unused variables** — All variables are used or prefixed with underscore if intentionally unused
3. **No unused imports** — All `use` statements are referenced in code
4. **No unreachable code paths** — All code paths are reachable or marked with justification
5. **Dead code attributes justified** — All `#[allow(dead_code)]` attributes have explanatory comments

---

## Phase 20: Unused File Cleanup

**Goal:** Clean up orphaned documentation, empty files, and unused documentation artifacts

**Phase Number:** 20  
**Requirements:** FILE-01, FILE-02, FILE-03, FILE-04, FILE-05  
**Dependencies:** Phase 19 (clean code first, then clean files)  
**Status:** ✅ COMPLETE (2026-02-23)

### Success Criteria

1. **All documentation files referenced** — Every file in `docs/` is included in Antora navigation or build output
2. **No empty files in repository** — All `.adoc`, `.md`, `.rs` files contain content (except intentional placeholders with comments)
3. **CLAUDE.md files are active** — All `CLAUDE.md` files are referenced by AI workflows and contain current information
4. **README.md files are current** — All `README.md` files are up to date and not orphaned from parent projects
5. **Non-build documentation reviewed** — Documentation not part of Antora build output is either integrated or removed

---

## Phase 21: Rust Documentation

**Goal:** Achieve comprehensive Rust documentation for all public APIs with clean `cargo doc` generation

**Phase Number:** 21  
**Requirements:** DOC-01, DOC-02, DOC-03, DOC-04, DOC-05, DOC-06, DOC-07, DOC-08  
**Dependencies:** Phase 20 (clean files first to avoid documenting what gets deleted)  

### Success Criteria

1. **All public modules documented** — Every public module has module-level documentation (`//!`)
2. **All public functions documented** — Every public function has doc comments (`///`) with description
3. **All public types documented** — Every public struct and enum has doc comments with field descriptions
4. **All trait implementations documented** — Trait `impl` blocks have documentation explaining the implementation
5. **Complex logic explained** — Functions with complex logic have inline comments explaining the "why"
6. **Clean `cargo doc` generation** — `cargo doc` completes with no warnings or broken links
7. **Public API examples** — Key public APIs include usage examples in doc comments
8. **Safety and error documentation** — Functions that panic, return errors, or use unsafe have appropriate sections

---

## Phase 22: Dependency Audit

**Goal:** Audit and clean up unused dependencies, features, and duplicates

**Phase Number:** 22  
**Requirements:** DEPS-01, DEPS-02, DEPS-03  
**Dependencies:** Phase 21 (final cleanup phase after all code is documented)  

### Success Criteria

1. **All dependencies used** — Every dependency in `Cargo.toml` is actually used in the codebase
2. **All features used** — Every feature flag in `Cargo.toml` is actually exercised
3. **No duplicate dependencies** — Dependencies are deduplicated where avoidable (e.g., multiple versions)

---

## Summary

| Phase | Name | Goal | Requirements | Dependencies |
|-------|------|------|--------------|--------------|
| 18 | Fix Compiler & Clippy Warnings | Zero warnings from rustc and clippy | LINT-01 to LINT-05 | None |
| 19 | Dead Code Elimination | Remove all dead code | DEAD-01 to DEAD-05 | Phase 18 |
| 20 | Unused File Cleanup | Clean up orphaned files | FILE-01 to FILE-05 | Phase 19 |
| 21 | Rust Documentation | Document all public APIs | DOC-01 to DOC-08 | Phase 20 |
| 22 | Dependency Audit | Clean up unused dependencies | DEPS-01 to DEPS-03 | Phase 21 |

**Total:** 5 phases, 24 requirements, 23 success criteria

---

## Execution Notes

### Phase Ordering Rationale

1. **Phase 18 (Lint) first:** Fix active code issues to get a clean baseline
2. **Phase 19 (Dead code) second:** After linting, actual dead code becomes visible
3. **Phase 20 (File cleanup) third:** Clean files after code is cleaned (avoid deleting files that might be needed)
4. **Phase 21 (Documentation) fourth:** Document what remains after cleanup
5. **Phase 22 (Dependencies) last:** Audit deps after all code changes are complete

### Expected Effort

- Phase 18: 2-4 hours (fix warnings)
- Phase 19: 2-3 hours (identify and remove dead code)
- Phase 20: 1-2 hours (file cleanup)
- Phase 21: 4-6 hours (comprehensive documentation)
- Phase 22: 1-2 hours (dependency audit)

**Total estimated time:** 10-17 hours across all phases

---

_Last updated: 2026-02-22_
