# Phase 07: Code Quality - Research

**Researched:** 2026-02-16  
**Domain:** Rust code quality refactoring patterns  
**Confidence:** HIGH

## Summary

Phase 7 requires refactoring the Rust codebase in `pkgs/artifacts/src/` to improve readability. The main issues identified are:

1. **Long functions**: Multiple functions exceed 50 lines (QUAL-05 violation)
2. **Deep call chains**: Some functions have 3+ levels of nesting
3. **Repetitive patterns**: Shared and non-shared artifact handlers have significant code duplication
4. **Large files**: `update.rs` is 1219 lines

**Primary recommendation:** Incremental refactoring approach:
1. First, split long functions by extracting cohesive sub-functions
2. Flatten deep nesting by using early returns and helper functions
3. Extract common patterns between shared/non-shared handlers
4. Use Rust's `Result` combinators to reduce nesting

## Code Quality Issues Found

### File: `src/backend/serialization.rs` (592 lines)

**Long Functions (>50 lines):**
- `run_serialize()` - ~98 lines (lines 35-134)
- `run_shared_serialize()` - ~125 lines (lines 143-261)
- `run_check_serialization()` - ~160 lines (lines 268-434)
- `run_shared_check_serialization()` - ~150 lines (lines 441-591)

**Issues:**
- Repetitive JSON file creation logic (machines.json, users.json)
- Similar error handling patterns across all functions
- Command building and execution is verbose

### File: `src/tui/effect_handler.rs` (226 lines)

**Long Functions:**
- `run_generator_and_store_output()` - ~44 lines (lines 54-98)
- `serialize_generated_output_to_backend()` - ~30 lines
- `execute()` - ~62 lines (lines 164-225) with deep match nesting

### File: `src/app/update.rs` (1219 lines)

**Long Functions:**
- `update()` - ~62 lines with deep pattern matching (lines 35-97)
- `update_artifact_list()` - ~26 lines (acceptable)
- `start_generation_for_selected()` - ~49 lines (lines 128-179)
- `update_prompt()` - ~45 lines (acceptable)
- `handle_prompt_enter()` - ~25 lines (acceptable)
- `handle_prompt_ctrl_d()` - ~22 lines (acceptable)
- `finish_prompts_and_start_generation()` - ~42 lines (lines 284-326)
- `handle_check_result()` - ~50 lines (lines 348-402)
- `handle_generator_finished()` - ~77 lines (lines 404-481) ⚠️
- `handle_serialize_finished()` - ~62 lines (lines 483-546) ⚠️
- `handle_shared_generator_finished()` - ~74 lines (lines 645-721) ⚠️
- `handle_shared_serialize_finished()` - ~62 lines (lines 723-786) ⚠️
- `update_generator_selection()` - ~84 lines (lines 550-641) ⚠️

**Duplicate Code Patterns:**
- `handle_generator_finished` and `handle_shared_generator_finished` share ~40 lines of nearly identical log handling
- `handle_serialize_finished` and `handle_shared_serialize_finished` share ~50 lines of identical log accumulation and error formatting

## Refactoring Patterns

### Pattern 1: Extract Helper Functions

**Before:**
```rust
fn handle_generator_finished(model, artifact_index, result) -> (Model, Effect) {
    match result {
        Ok(output) => {
            // 30 lines of log handling
            // 20 lines of status updates
            // 10 lines of effect building
        }
        Err(e) => {
            // 25 lines of error handling
        }
    }
}
```

**After:**
```rust
fn handle_generator_finished(model, artifact_index, result) -> (Model, Effect) {
    match result {
        Ok(output) => handle_generator_success(model, artifact_index, output),
        Err(error) => handle_generator_failure(model, artifact_index, error),
    }
}

fn handle_generator_success(model, artifact_index, output) -> (Model, Effect) {
    store_generator_logs(&mut model, artifact_index, &output);
    update_generation_step(&mut model, GenerationStep::Serializing);
    build_serialize_effect(&model, artifact_index)
}

fn handle_generator_failure(model, artifact_index, error) -> (Model, Effect) {
    store_generator_error(&mut model, artifact_index, &error);
    (model, Effect::None)
}
```

### Pattern 2: Flatten Match Nesting

**Before:**
```rust
fn long_function() -> Result<()> {
    let x = get_value()?;
    match x {
        Some(y) => {
            match y.process() {
                Ok(z) => {
                    match z.validate() {
                        Ok(_) => do_something(),
                        Err(e) => handle_error(e),
                    }
                }
                Err(e) => handle_error(e),
            }
        }
        None => handle_none(),
    }
}
```

**After:**
```rust
fn long_function() -> Result<()> {
    let x = get_value()?;
    let y = x.ok_or_else(|| Error::missing_value())?;
    let z = y.process().map_err(handle_processing_error)?;
    z.validate()?;
    do_something()
}
```

### Pattern 3: Extract Common Error Handling

Both serialize handlers share this pattern:
```rust
let mut output = String::new();
for log in &entry.step_logs().check {
    output.push_str(&format!("[check] {}\n", log.message));
}
for log in &entry.step_logs().generate {
    output.push_str(&format!("[generate] {}\n", log.message));
}
```

**Extract to helper:**
```rust
fn format_step_logs(entry: &ArtifactEntry) -> String {
    let mut output = String::new();
    for log in &entry.step_logs().check {
        output.push_str(&format!("[check] {}\n", log.message));
    }
    for log in &entry.step_logs().generate {
        output.push_str(&format!("[generate] {}\n", log.message));
    }
    output
}
```

### Pattern 4: Use Iterator Methods

**Before:**
```rust
let mut stdout_lines = Vec::new();
let mut stderr_lines = Vec::new();

for line in &captured.lines {
    match line.stream {
        OutputStream::Stdout => stdout_lines.push(line.content.clone()),
        OutputStream::Stderr => stderr_lines.push(line.content.clone()),
    }
}
```

**After:**
```rust
let stdout_lines: Vec<String> = captured
    .lines
    .iter()
    .filter(|l| l.stream == OutputStream::Stdout)
    .map(|l| l.content.clone())
    .collect();

let stderr_lines: Vec<String> = captured
    .lines
    .iter()
    .filter(|l| l.stream == OutputStream::Stderr)
    .map(|l| l.content.clone())
    .collect();
```

## Tools for Code Quality

### Clippy (Already in use)

The project already uses clippy. Add these additional lints to enforce code quality:

```toml
# In Cargo.toml [lints.clippy] section
single_call_fn = "deny"
too_many_lines = "deny"
unnecessary_wraps = "warn"
cognitive_complexity = "warn"
```

### Manual Checks

Before committing refactored code:

1. **Line count:**
   ```bash
   cargo clippy -- -D clippy::too_many_lines
   ```

2. **Function complexity:**
   ```bash
   cargo bloat --release --crates  # Check largest functions
   ```

3. **Naming conventions:**
   ```bash
   grep -rn "\b\w\{2,3\}\b" pkgs/artifacts/src/ | grep -v "as\|if\|in\|fn\|use\|mod\|let\|mut\|pub\|self\|Ok\|Err\|Box\|Vec\|str\|u32\|i32"
   ```

## Refactoring Order

**Wave 1: Split Long Functions** (Highest impact)
1. `update.rs`: Split handler functions into success/failure helpers
2. `serialization.rs`: Extract JSON file creation and command building
3. `effect_handler.rs`: Extract output splitting and temp directory management

**Wave 2: Deduplicate Patterns**
1. Extract shared error formatting in update.rs
2. Extract shared log accumulation patterns
3. Create common serialization helpers

**Wave 3: Flatten Call Chains**
1. Replace nested match with Result combinators where appropriate
2. Extract early return patterns
3. Use `?` operator more consistently

**Wave 4: Rename Abbreviations**
1. Find and rename abbreviated variables (cfg, ctx, hdl)
2. Update function names to be descriptive
3. Ensure all names are 3+ words where possible

## Common Pitfalls

### Pitfall 1: Breaking Tests During Refactor

**What goes wrong:** Extracting functions changes call sites and breaks existing tests.

**How to avoid:**
- Keep function signatures stable during Wave 1
- Only split internal implementation
- Run `cargo test` after each function extraction
- Use `#[inline]` temporarily if needed

### Pitfall 2: Over-Extraction

**What goes wrong:** Extracting too many tiny functions makes code harder to follow.

**How to avoid:**
- Each extracted function should have a single, clear responsibility
- If the function name is longer than the code, don't extract
- Group related operations in cohesive helpers

### Pitfall 3: Deep Borrow Issues

**What goes wrong:** After splitting functions, borrow checker complains about mutable borrows.

**Example:**
```rust
// After split, this pattern breaks:
fn update(model: Model) {
    helper_a(&mut model);  // immutable borrow of model.entries
    helper_b(&mut model);  // error: cannot borrow mutably twice
}
```

**How to avoid:**
- Pass individual fields instead of whole model when possible
- Use `take()` patterns: `let entries = std::mem::take(&mut model.entries)`
- Refactor data flow before splitting functions

### Pitfall 4: Losing Comments

**What goes wrong:** Important comments explaining "why" get lost during extraction.

**How to avoid:**
- Move comments with the code they describe
- Add doc comments to extracted functions explaining context
- Keep architectural comments at the top level

## Files to Refactor

| File | Priority | Main Issues |
|------|----------|-------------|
| `src/app/update.rs` | P0 | Multiple 60+ line functions, duplicate patterns |
| `src/backend/serialization.rs` | P0 | Long functions, repetitive JSON creation |
| `src/tui/effect_handler.rs` | P1 | execute() has deep nesting |
| `src/backend/generator.rs` | P2 | To be analyzed |
| `src/config/make.rs` | P2 | To be analyzed |
| `src/config/backend.rs` | P2 | To be analyzed |

## Specific Function Targets

### High Priority (>70 lines)

1. `src/app/update.rs:handle_generator_finished` (77 lines)
2. `src/app/update.rs:update_generator_selection` (84 lines)
3. `src/backend/serialization.rs:run_check_serialization` (160 lines)
4. `src/backend/serialization.rs:run_shared_check_serialization` (150 lines)
5. `src/backend/serialization.rs:run_shared_serialize` (125 lines)

### Medium Priority (50-70 lines)

1. `src/app/update.rs:handle_serialize_finished` (62 lines)
2. `src/app/update.rs:handle_shared_generator_finished` (74 lines)
3. `src/app/update.rs:handle_shared_serialize_finished` (62 lines)
4. `src/app/update.rs:update` (62 lines)
5. `src/tui/effect_handler.rs:execute` (62 lines)

## Refactoring Checklist

For each function being refactored:

- [ ] Function under 50 lines after refactoring
- [ ] No nested calls deeper than 2 levels (f(g(x)))
- [ ] Clear, unabbreviated names
- [ ] Single responsibility
- [ ] All tests pass
- [ ] No clippy warnings

## Metadata

**Confidence breakdown:**

- Code structure: HIGH - Clear from reading source
- Refactoring patterns: HIGH - Standard Rust idioms
- Tool recommendations: MEDIUM-HIGH - Based on current Cargo.toml

**Research date:** 2026-02-16
**Valid until:** 2026-03-16
