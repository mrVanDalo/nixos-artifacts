# Phase 7: Code Quality - Context

**Gathered:** 2026-02-17 **Status:** Ready for planning

<domain>
## Phase Boundary

Refactor the Rust codebase in `pkgs/artifacts/src/` to improve readability by
flattening deep call chains and eliminating abbreviated names. Target: No
function chains deeper than 2 levels (f(g(x)) allowed, f(g(h(x))) not allowed),
all names descriptive and unabbreviated, all functions under 50 lines with
single clear responsibility.

</domain>

<decisions>
## Implementation Decisions

### Refactoring Order

- **Easy wins first** — Start with simple files to establish patterns before
  tackling complex ones
- **Document patterns in RESEARCH.md** — Add guidance on naming conventions,
  call chain flattening, and function splitting to the research file
- **Full directory scope** — All files in `pkgs/artifacts/src/` as specified in
  QUAL-07 requirement
- **1-2 files per plan** — Focused, thorough refactoring rather than broad but
  shallow changes

### Naming Conventions

- **Follow Rust naming guidelines** — Use
  https://rust-lang.github.io/api-guidelines/naming.html as the authority
- **Change all abbreviations** — Including standard Rust ones like `ctx`, `cfg`,
  `hdl` unless they are keywords (`mut`, `async`, `await`)
- **Acronyms follow Rust style** — Use `Url`, `Http` instead of `URL`, `HTTP`
- **Descriptive loop variables** — Use `index`, `count`, `total` instead of `i`,
  `n`, `len`
- **Single-word exceptions allowed** — Common operations like `run`, `get`,
  `set` are acceptable for very standard patterns

### Function Splitting Strategy

- **Split by responsibility** — Each function should have one clear, single
  responsibility
- **Keep helpers in same file** — Private helper functions stay near the
  functions that use them
- **Error handling inline** — Use `?` operator and early returns, keep error
  handling inline rather than extracting wrappers
- **50 line maximum** — No function should exceed 50 lines after splitting

### Claude's Discretion

- **Call chain style** — Claude to decide between explicit intermediate
  variables vs Option/Result combinators vs other patterns based on context and
  readability
- **Helper function naming** — Descriptive names for extracted functions
- **Exact splitting points** — Where to split long functions based on
  responsibility boundaries
- **File order within plans** — Which specific files to tackle in which order
  within the 1-2 file constraint

</decisions>

<specifics>
## Specific Ideas

- "Follow this guideline https://rust-lang.github.io/api-guidelines/naming.html"
  — Use as the naming authority
- No specific file references given — open to Claude's analysis of which files
  are easiest wins
- Focus on readability over cleverness — clear intent should be immediately
  obvious

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

_Phase: 07-code-quality_ _Context gathered: 2026-02-17_
