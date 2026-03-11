# Phase 8: Smart Logging - Context

**Gathered:** 2026-02-17 **Status:** Ready for planning (REVISED)

<domain>
## Phase Boundary

Replace hardcoded debug logging with an opt-in CLI argument
(`--log-file <path>`) that writes comprehensive debug information to a specified
file when provided, and produces zero logging output when omitted. The logging
system supports four configurable levels (ERROR, WARN, INFO, DEBUG) via
grep-able macro API. Feature-gated: `--features logging` enables full
functionality; without it, macros are no-ops and CLI args are hidden. Default in
Nix package has logging enabled.

When logging is not enabled: TUI runs normally with no stdout/stderr output
(would break TUI), only generator/serialization script output shown in TUI.

</domain>

<decisions>
## Implementation Decisions

### Log Levels

- Four levels: ERROR, WARN, INFO, DEBUG (uppercase in output)
- Default level when `--log-file` is provided: DEBUG (all messages logged)
- Configurable via `--log-level` flag to filter which levels are written
- Filter behavior: minimum level and above (e.g., --log-level INFO shows
  INFO+WARN+ERROR)

### Log Content by Level

**ERROR:**

- Unexpected failures and panics
- Backend script execution failures
- Serialization/deserialization errors
- File I/O errors that prevent operation

**WARN:**

- Non-fatal issues that don't stop execution
- Deprecated feature usage
- Potential misconfigurations
- Recoverable errors (e.g., retry scenarios)

**INFO:**

- User-facing progress milestones
- High-level lifecycle events ("Starting generation", "Serialization complete")
- Operation summaries (counts, durations)
- State transitions at coarse granularity

**DEBUG:**

- Channel message details (sent/received)
- Effect execution parameters and results
- Backend call parameters (scripts, environment variables)
- Timestamps with millisecond precision
- Temporary file paths
- Generator stdout/stderr paths and exit codes
- Prompt names (but NOT values - redacted for security)
- Module path included at DEBUG level only

### Macro API Design

- Macro names: `error!`, `warn!`, `info!`, `debug!`
- Must be easily grep-able: `grep -r 'debug!' src/`
- Format strings only: `debug!("message: {}", variable)` (no built-in key-value)
- Automatically include module path at DEBUG level only
- Include source line numbers at DEBUG level only
- Zero-cost when disabled via feature flag

### Feature Flag Behavior

**With `--features logging` (Nix default):**

- All macros functional
- CLI args `--log-file` and `--log-level` available
- Full logging infrastructure compiled in

**Without `--features logging` (cargo default):**

- All macro calls compile to no-ops (no runtime cost)
- CLI args `--log-file` and `--log-level` removed/hidden
- No logging code included in binary

### CLI Flag Behavior

**`--log-file <path>`:**

- Required to enable any logging
- Absolute or relative path accepted
- Overwrite existing file (not append) — fresh logs each run
- Create parent directories if needed
- Error if path is a directory
- File permissions: 640 (owner + group read, others no access)

**`--log-level <level>`:**

- Optional, defaults to "DEBUG" when `--log-file` is set
- Valid values: ERROR, WARN, INFO, DEBUG (uppercase)
- Filters which messages are written (level and above)

**When `--log-file` is omitted:**

- Zero logging to stdout/stderr (silent operation)
- TUI runs normally with visual feedback
- Generator/serialization script output still shown in TUI
- All existing `println!`/`eprintln!` debug statements removed

### Log Format Structure

**Text format (default):**

- Human-readable structured text format
- Timestamp: ISO8601 with milliseconds (e.g., 2024-01-20T14:32:10.123Z)
- Log level in brackets: [DEBUG], [INFO], [WARN], [ERROR]
- Module path included
- Field order: [TIMESTAMP] [LEVEL] MODULE: Message
- Structured key-value pairs on indented continuation lines
- Skip empty or None fields

**Example:**

```
[2024-01-20T14:32:10.123Z] [DEBUG] artifacts::effect_handler: Running generator script
  artifact: "ssh-key"
  machine: "server-1"
  script: "/nix/store/...-generator.sh"
  
[2024-01-20T14:32:10.456Z] [INFO] artifacts::cli: Artifact generation complete
  duration_ms: 2345
  artifacts_generated: 3
```

**JSON format (optional via `--log-format json`):**

- Machine-parseable JSON objects
- Same information as text format
- One JSON object per log entry

### File Write Behavior

**Startup validation (fail fast):**

- Test file writability immediately on startup
- If can't write: fail fast with clear error (don't start operation)
- Check: path not directory, parent dirs creatable, writable permissions

**Runtime behavior:**

- Stream logs in real-time (don't buffer until end)
- Flush after each log entry
- If write fails mid-operation: retry with exponential backoff
- After retries exhausted: fail entire application

### Scope & Existing Code

**What gets replaced:**

- Hardcoded `/tmp/artifacts_debug.log` path - **REMOVE entirely**
- All existing `println!`/`eprintln!` debug statements - **convert to macros or
  remove**

**What stays:**

- TUI visual output (essential for user interaction)
- Generator/serialization script output shown in TUI
- Error messages to stderr (for actual errors, not logs)

### Security Considerations

- Never log sensitive values (prompts, secrets, passwords)
- Log prompt names but redact values
- At error level: include context without exposing secrets
- Environment variables: log names only, not values
- File permissions 640 restrict log file access

### Claude's Discretion

- Choice of logging crate (`log` + custom implementation vs `tracing`)
- Internal buffering strategy for performance
- Exact retry backoff parameters (exponential backoff formula)
- Exact implementation of feature-gated macro no-ops
- JSON format field naming convention

</decisions>

<specifics>
## Specific Ideas

- "I want macros like error!("message", ...) or debug!("message something", ...)
  so I can easily spot in the code when there's a log message"
- Use four distinct log levels: ERROR, WARN, INFO, DEBUG
- Feature flag controls everything: with feature = full logging, without =
  no-ops + hidden CLI
- Nix package enables logging feature by default
- Remove ALL println!/eprintln! debug output (breaks TUI)
- Only TUI + script output visible without --log-file
- Test file writability at startup (fail fast)
- Retry with backoff at runtime, ultimately fail app

</specifics>

<deferred>
## Deferred Ideas

- Remote logging (syslog, HTTP endpoints) — separate phase
- Log rotation / retention policies — future enhancement
- Metrics/monitoring integration — separate concern
- Async-specific logging optimizations — revisit if performance issues

</deferred>

---

_Phase: 08-smart-logging_ _Context gathered: 2026-02-17 (REVISED)_
