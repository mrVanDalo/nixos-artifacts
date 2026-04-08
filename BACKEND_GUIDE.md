# Backend Developer Guide for nixos-artifacts

**Version:** 4.0\
**Last Updated:** 2026-04-08\
**Status:** Complete reference for backend implementation

This guide provides everything needed to implement a custom serialization
backend for nixos-artifacts. It is designed to be copied standalone to other
repositories.

## Table of Contents

- [Overview](#overview)
- [Backend Interface](#backend-interface)
- [Backend Scripts](#backend-scripts)
  - [check](#check)
  - [serialize](#serialize)
- [Backend Configuration (backend.toml)](#backend-configuration-backendtoml)
- [Environment Variables Reference](#environment-variables-reference)
- [File Format Reference](#file-format-reference)
- [Complete Working Example](#complete-working-example)
- [Error Handling](#error-handling)
- [Testing](#testing)
- [Troubleshooting](#troubleshooting)
- [See Also](#see-also)

## Overview

### What is nixos-artifacts?

**nixos-artifacts** is a framework that unifies handling of artifacts (secrets
and generated files) in NixOS flakes through a common abstraction over multiple
backends. It separates the concerns of:

- **Generation**: Creating secret files (SSH keys, SSL certificates, passwords)
- **Serialization**: Storing those files securely (encryption, cloud services,
  version control)
- **Deserialization**: Retrieving files during system activation

### What is a Backend?

A **backend** is a set of shell scripts that implement the storage contract for
artifact files. When you run `artifacts generate`, the CLI orchestrates:

1. **Check Phase**: Calls `check` to see if the artifact already exists
2. **Generation Phase**: Runs generator scripts to create files (if needed)
3. **Serialization Phase**: Calls your backend's `serialize` script to store the
   files
4. **Deserialization Phase** (later): Happens during system activation via NixOS
   modules

### Backend Types

Backends handle three types of targets:

| Type                   | Description                              | Permissions                           |
| ---------------------- | ---------------------------------------- | ------------------------------------- |
| **NixOS Machines**     | Full system configurations               | Has `owner` and `group` fields        |
| **Home Manager Users** | User-level configurations                | `owner` and `group` are `null`        |
| **Shared Artifacts**   | Identical across multiple machines/users | Requires dedicated `shared.*` scripts |

### When to Write a Custom Backend

Consider writing a custom backend when:

- You want to integrate with a secret management system not yet supported
- Your organization has specific compliance or storage requirements
- You need custom encryption or access control mechanisms
- You want to store artifacts in a proprietary or internal system

## Backend Interface

The CLI calls your backend scripts at specific points in the artifact lifecycle.
All scripts use a **unified interface** regardless of context (nixos, homemanager,
or shared):

- Receive data through unified environment variables
- Must return exit code 0 on success, non-zero on failure
- Read target information from a single `$targets` JSON file

Note: Generator scripts run in isolated bubblewrap containers for security, but
backend serialization scripts run directly on the host system.

### Script Lifecycle

```
check
    ↓ (if artifact doesn't exist)
generator runs → creates files in $out
    ↓
serialize stores files from $out
```

The same scripts can handle all contexts (nixos, homemanager, shared) since they
receive context information in `$targets`. You can use the same script for all
target types or provide separate scripts per target type.

## Backend Scripts

All backend scripts use a **unified interface** with the same environment
variables regardless of context (nixos, homemanager, or shared). This means
you can use the same script for all target types.

### check

**Purpose:** Determine whether an artifact already exists in your backend's
storage. This prevents accidental overwrites of existing secrets.

**When Called:** Before the generator script runs, during the
`artifacts generate` workflow.

**Exit Codes:**

| Exit Code | Meaning                   | Action Taken by CLI               |
| --------- | ------------------------- | --------------------------------- |
| 0         | Artifact exists           | Skip generation for this artifact |
| Non-zero  | Artifact needs generation | Continue to generator phase       |

**Environment Variables:**

| Variable            | Type      | Description                                                | Example                     |
| ------------------- | --------- | ---------------------------------------------------------- | --------------------------- |
| `$artifact`         | String    | Artifact name being processed                              | `ssh-host-key`              |
| `$artifact_context` | String    | Context type: `"nixos"`, `"homemanager"`, or `"shared"`    | `nixos`                     |
| `$targets`          | File      | Path to JSON file with target information                  | `/tmp/targets-xxx.json`     |
| `$inputs`           | Directory | Path to directory containing JSON files with file metadata | `/tmp/artifacts-inputs-xxx` |
| `$LOG_LEVEL`        | String    | Log level for the script                                   | `info`                      |

**Example:**

```bash
#!/usr/bin/env bash
set -euo pipefail

# Parse context and target info from unified targets.json
context=$(jq -r '.context' "$targets")

# For single targets (nixos or homemanager), there's one entry
# For shared, iterate all targets
if [[ "$context" == "shared" ]]; then
    # Check all targets for shared artifacts
    for target in $(jq -r '.targets[].name' "$targets"); do
        target_config=$(jq -r ".targets[] | select(.name == \"$target\") | .config" "$targets")
        storage_path=$(echo "$target_config" | jq -r '.storage_path // "/var/lib/mybackend"')

        if [[ ! -f "$storage_path/$target/$artifact.json" ]]; then
            exit 1  # Missing for this target
        fi
    done
    exit 0  # All targets have the artifact
else
    # Single target (nixos or homemanager)
    target_name=$(jq -r '.targets[0].name' "$targets")
    target_config=$(jq -r '.targets[0].config' "$targets")
    storage_path=$(echo "$target_config" | jq -r '.storage_path // "/var/lib/mybackend"')

    if [[ -f "$storage_path/$target_name/$artifact.json" ]]; then
        exit 0  # Artifact exists
    fi
    exit 1  # Needs generation
fi
```

### serialize

**Purpose:** Store the files generated by the generator script in your backend's
storage.

**When Called:** After the generator script successfully completes, for each
target that needs the artifact.

**$out Directory Contents:**

The `$out` directory contains files created by the generator script. Filenames
match the keys in the artifact's `files` attribute. For example, if your
artifact defines:

```nix
files = {
  cert = { path = "/etc/ssl/cert.pem"; owner = "root"; };
  key = { path = "/etc/ssl/key.pem"; owner = "root"; };
};
```

Then `$out` will contain:

- `$out/cert` - The certificate content
- `$out/key` - The private key content

**Expected Behavior:**

- Store all files from `$out` in your backend
- Return exit code 0 on success, non-zero on failure
- The CLI aborts if this script fails

**Environment Variables:**

| Variable            | Type      | Description                                             | Example                  |
| ------------------- | --------- | ------------------------------------------------------- | ------------------------ |
| `$artifact`         | String    | Artifact name being processed                           | `api-key`                |
| `$artifact_context` | String    | Context type: `"nixos"`, `"homemanager"`, or `"shared"` | `nixos`                  |
| `$targets`          | File      | Path to JSON file with target information               | `/tmp/targets-xxx.json`  |
| `$out`              | Directory | Path to directory containing all generated files        | `/tmp/artifacts-out-xxx` |
| `$LOG_LEVEL`        | String    | Log level for the script                                | `info`                   |

**Example:**

```bash
#!/usr/bin/env bash
set -euo pipefail

# Parse context from unified targets.json
context=$(jq -r '.context' "$targets")

# Function to store files for a target
store_files() {
    local target_name="$1"
    local target_config="$2"
    local storage_path=$(echo "$target_config" | jq -r '.storage_path // "/var/lib/mybackend"')

    mkdir -p "$storage_path/$target_name"

    for file in "$out"/*; do
        if [[ -f "$file" ]]; then
            local filename=$(basename "$file")
            cp "$file" "$storage_path/$target_name/$artifact-$filename"
            echo "Stored: $target_name/$artifact-$filename"
        fi
    done
}

# Process all targets (works for both single and shared)
for row in $(jq -c '.targets[]' "$targets"); do
    target_name=$(echo "$row" | jq -r '.name')
    target_config=$(echo "$row" | jq -c '.config')
    store_files "$target_name" "$target_config"
done
```

## Backend Configuration (backend.toml)

The `backend.toml` file defines your backend's scripts using a nested,
target-centric structure.

### Complete TOML Structure

```toml
# NixOS target configuration (required)
[mybackend.nixos]
enabled = true                    # Optional, inferred from scripts presence
check = "./scripts/nixos_check.sh"
serialize = "./scripts/nixos_serialize.sh"

# Home Manager target configuration (required)
[mybackend.home]
enabled = true
check = "./scripts/home_check.sh"
serialize = "./scripts/home_serialize.sh"

# Shared artifact configuration (optional - for shared artifacts)
[mybackend.shared]
enabled = true
check = "./scripts/shared_check.sh"
serialize = "./scripts/shared_serialize.sh"

# Backend-specific settings (optional)
[mybackend.settings]
storage_path = "/var/lib/mybackend"
encryption = "aes256-gcm"
```

### Validation Rules

The `check` and `serialize` scripts must be provided together or both omitted:

| `check` | `serialize` | Result                                |
| ------- | ----------- | ------------------------------------- |
| absent  | absent      | Valid: `enabled = true` (passthrough) |
| present | present     | Valid: `enabled = true`, `serializes` |
| present | absent      | **ERROR**: "check requires serialize" |
| absent  | present     | **ERROR**: "serialize requires check" |

### enabled Inference

The `enabled` field is inferred if not explicitly set:

| Condition                                        | Inferred `enabled` | Inferred `serializes` |
| ------------------------------------------------ | ------------------ | --------------------- |
| Section absent                                   | `false`            | N/A                   |
| Section present, no scripts, no `enabled`        | `false` (implicit) | `false`               |
| Section present, no scripts, `enabled = true`    | `true` (explicit)  | `false`               |
| Section present, both scripts, no `enabled`      | `true` (default)   | `true`                |
| Section present, both scripts, `enabled = true`  | `true` (explicit)  | `true`                |
| Section present, both scripts, `enabled = false` | `false` (explicit) | `true`                |

### supports_shared Inference

A backend supports shared artifacts if:

- The `[backend.shared]` section exists AND
- `enabled = true` (explicit or inferred)

### Target-Specific Scripts

You can define different scripts for NixOS and Home Manager targets:

```toml
[mybackend.nixos]
check = "./nixos_check.sh"
serialize = "./nixos_serialize.sh"

[mybackend.home]
check = "./home_check.sh"
serialize = "./home_serialize.sh"
```

All scripts receive the same unified environment variables (`$artifact`,
`$artifact_context`, `$targets`, etc.). Use `$artifact_context` or parse
`$targets` to determine the target type if needed.

If you only define one target type, that backend only works with that
configuration type.

### Include Directive

Backend configurations can be split across multiple files:

```toml
# backend.toml
include = ["./backends/agenix.toml", "./backends/sops.toml"]

[my-custom.nixos]
check = "./check.sh"
serialize = "./serialize.sh"

[my-custom.home]
check = "./check.sh"
serialize = "./serialize.sh"
```

Paths in `include` are resolved relative to the file containing the directive.
Nested includes are supported, and circular includes are detected and rejected.

## Environment Variables Reference

All backend scripts (check and serialize) receive the same unified environment
variables, regardless of context (nixos, homemanager, or shared):

### Environment Variables by Script Type

| Variable            | check | serialize |
| ------------------- | ----- | --------- |
| `$artifact`         | ✅    | ✅        |
| `$artifact_context` | ✅    | ✅        |
| `$targets`          | ✅    | ✅        |
| `$inputs`           | ✅    | ❌        |
| `$out`              | ❌    | ✅        |
| `$LOG_LEVEL`        | ✅    | ✅        |

### Variable Details

| Variable            | Type      | Description                                                          | Example                        |
| ------------------- | --------- | -------------------------------------------------------------------- | ------------------------------ |
| `$artifact`         | String    | The artifact name being processed                                    | `ssh-host-key`                 |
| `$artifact_context` | String    | Context type: `"nixos"`, `"homemanager"`, or `"shared"`              | `nixos`                        |
| `$targets`          | File      | Path to JSON file containing target information and backend configs  | `/tmp/targets-abc123.json`     |
| `$inputs`           | Directory | Contains JSON files with file metadata (one per artifact file)       | `/tmp/artifacts-inputs-def456` |
| `$out`              | Directory | Contains generated files from the generator script                   | `/tmp/artifacts-out-abc123`    |
| `$LOG_LEVEL`        | String    | Log level for script output                                          | `info`                         |

## File Format Reference

### $targets JSON

The `$targets` file is a unified JSON structure containing context and target
information:

```json
{
  "context": "nixos" | "homemanager" | "shared",
  "targets": [
    {
      "name": "target-name",
      "type": "nixos" | "homemanager",
      "config": { ... backend settings ... }
    }
  ]
}
```

**For single targets (nixos or homemanager):**

There is exactly one entry in the `targets` array:

```json
{
  "context": "nixos",
  "targets": [
    {
      "name": "server-one",
      "type": "nixos",
      "config": {
        "storage_path": "/var/lib/secrets",
        "encryption_key": "abc123"
      }
    }
  ]
}
```

**For Home Manager targets:**

```json
{
  "context": "homemanager",
  "targets": [
    {
      "name": "alice@workstation",
      "type": "homemanager",
      "config": {
        "storage_path": "~/.local/share/secrets"
      }
    }
  ]
}
```

**For shared artifacts:**

All machines and users that share the artifact are listed:

```json
{
  "context": "shared",
  "targets": [
    {
      "name": "server-one",
      "type": "nixos",
      "config": {
        "storage_path": "/var/lib/secrets",
        "encryption_key": "abc123"
      }
    },
    {
      "name": "server-two",
      "type": "nixos",
      "config": {
        "storage_path": "/var/lib/secrets",
        "encryption_key": "def456"
      }
    },
    {
      "name": "alice@workstation",
      "type": "homemanager",
      "config": {
        "storage_path": "~/.local/share/secrets"
      }
    }
  ]
}
```

### $inputs Directory

Each file in `$inputs` is named after a file key and contains JSON with file
metadata:

**For NixOS artifacts:**

```json
{
  "path": "/etc/ssh/ssh_host_ed25519_key",
  "owner": "root",
  "group": "root"
}
```

**For Home Manager artifacts:**

```json
{
  "path": "~/.ssh/id_ed25519",
  "owner": null,
  "group": null
}
```

Note: The `owner` and `group` fields may be `null` for Home Manager artifacts
since home-manager doesn't manage system-level permissions. There is no `mode`
field.

## Complete Working Example

Here's a minimal but complete backend example that stores artifacts as tar.gz
archives. Because the interface is unified, the same scripts work for all
contexts (nixos, homemanager, and shared).

### Directory Structure

```
my-backend/
├── backend.toml
├── check.sh              # Unified check script for all contexts
└── serialize.sh          # Unified serialize script for all contexts
```

### backend.toml

```toml
# All target types use the same unified scripts
[mybackend.nixos]
check = "./check.sh"
serialize = "./serialize.sh"

[mybackend.home]
check = "./check.sh"
serialize = "./serialize.sh"

[mybackend.shared]
check = "./check.sh"
serialize = "./serialize.sh"
```

### check.sh

```bash
#!/usr/bin/env bash
set -euo pipefail

# Unified check script for all contexts (nixos, homemanager, shared)
#
# Environment variables available:
# $artifact         - Artifact name
# $artifact_context - "nixos", "homemanager", or "shared"
# $targets          - Path to JSON file with target information
# $inputs           - Directory with expected file metadata

STORAGE_DIR="${STORAGE_DIR:-./storage}"

# Check all targets in the targets.json file
for target_name in $(jq -r '.targets[].name' "$targets"); do
    ARTIFACT_FILE="$STORAGE_DIR/$target_name/$artifact.tar.gz"

    if [[ ! -f "$ARTIFACT_FILE" ]]; then
        echo "Missing: $ARTIFACT_FILE"
        exit 1  # Needs generation
    fi
done

echo "EXISTS"
exit 0
```

### serialize.sh

```bash
#!/usr/bin/env bash
set -euo pipefail

# Unified serialize script for all contexts (nixos, homemanager, shared)
#
# Environment variables available:
# $artifact         - Artifact name
# $artifact_context - "nixos", "homemanager", or "shared"
# $targets          - Path to JSON file with target information
# $out              - Directory containing generated files

STORAGE_DIR="${STORAGE_DIR:-./storage}"

# Store for all targets in the targets.json file
for target_name in $(jq -r '.targets[].name' "$targets"); do
    mkdir -p "$STORAGE_DIR/$target_name"

    # Archive all files from $out
    tar -czf "$STORAGE_DIR/$target_name/$artifact.tar.gz" -C "$out" .
    echo "Serialized: $target_name/$artifact.tar.gz"
done
```

### Alternative: Per-Target Scripts

If you need different behavior for different target types, you can still use
separate scripts:

```toml
[mybackend.nixos]
check = "./nixos_check.sh"
serialize = "./nixos_serialize.sh"

[mybackend.home]
check = "./home_check.sh"
serialize = "./home_serialize.sh"

[mybackend.shared]
check = "./shared_check.sh"
serialize = "./shared_serialize.sh"
```

Each script still receives the same unified environment variables. The
`$artifact_context` variable tells you which context you're running in, and
`$targets` contains the appropriate target entries.

## Error Handling

### Use Strict Mode

Always start scripts with:

```bash
#!/usr/bin/env bash
set -euo pipefail
```

This ensures:

- `-e`: Exit immediately on any command failure
- `-u`: Error on undefined variables
- `-o pipefail`: Pipeline fails if any command fails (not just the last)

### Meaningful Error Messages

When your script fails, print a clear message to stderr:

```bash
if [[ ! -f "$targets" ]]; then
    echo "Error: Targets file not found: $targets" >&2
    exit 1
fi
```

### Exit Code Conventions

While the CLI only checks zero vs non-zero, follow these conventions:

| Exit Code | Meaning                            |
| --------- | ---------------------------------- |
| 0         | Success                            |
| 1         | General error                      |
| 2         | Invalid arguments or configuration |
| 3         | Missing dependencies               |
| 127       | Command not found                  |

## Testing

### Testing Phases

1. **Single Artifact Test:** Define a simple artifact with one file, test
   generation and deployment.

2. **Shared Artifact Test:** Define a shared artifact across multiple machines
   or users.

3. **Mixed Target Test:** Test with both NixOS and Home Manager configurations.

### Debugging Tips

- Add `echo` statements to your scripts - they're captured in the TUI's log view
- Check the generated JSON files in temporary directories
- Use `set -x` for verbose trace output during development
- Test scripts manually with mocked environment variables

### Test with the CLI

```bash
# Set the backend config path and run from your flake directory
NIXOS_ARTIFACTS_BACKEND_CONFIG=/path/to/my-backend/backend.toml \
  artifacts /path/to/flake
```

## Troubleshooting

### "Backend does not support shared artifacts"

**Cause:** You tried to generate a shared artifact with a backend that doesn't
have a `[backend.shared]` section.

**Solution:** Either:

1. Add a `[backend.shared]` section with `check` and `serialize` scripts
2. Use a different backend for shared artifacts

### Exit Code Confusion

**Problem:** Your script returns 0 but the CLI thinks it failed (or vice versa).

**Cause:** The last command in your script determines the exit code.

**Solution:** Explicitly use `exit 0` or `exit 1` at the end of your script.

### Path Issues with $out and $inputs

**Problem:** Files aren't being found in `$out` or `$inputs`.

**Cause:** These directories contain files, not subdirectories. Iterate with
`for file in "$out"/*`.

**Solution:**

```bash
for file in "$out"/*; do
    if [[ -f "$file" ]]; then
        # Process file
    fi
done
```

### JSON Parsing Errors

**Problem:** `jq` commands fail with "parse error".

**Cause:** The `$targets` file might be empty or malformed.

**Solution:** Use the `//` operator to provide defaults when reading config:

```bash
# Read config with defaults
target_config=$(jq -r '.targets[0].config' "$targets")
STORAGE_PATH=$(echo "$target_config" | jq -r '.storage_path // "/default/path"')
```

### Permission Issues

**Problem:** Your script can't write to the storage directory.

**Solution:** Ensure your storage path is writable by the user running the CLI.
For the generator script (which runs in a bubblewrap container), paths under
`/var/lib`, `/tmp`, or explicitly configured paths are recommended. For
serialization scripts, these run without container isolation and use normal
filesystem permissions.

## See Also

- **Full Documentation:** See the Antora documentation site for comprehensive
  guides
- **Example Backends:** Check `pkgs/artifacts/examples/backends/` in the
  nixos-artifacts repository
- **CLI Reference:** Run `artifacts --help` for command-line options
- **Repository:** https://github.com/mrVanDalo/nixos-artifacts

## Using This Guide with AI Assistants

This guide is designed to be self-contained and can be used with AI coding
assistants (like Claude, ChatGPT, or GitHub Copilot) to implement backends
quickly. When working with an AI assistant:

1. **Copy this guide** into your conversation or provide a link to it
2. **Describe your storage backend** - Where do you want to store artifacts?
   (e.g., encrypted files in git, cloud secret manager, custom storage)
3. **Ask the AI to generate** the required scripts based on this guide's
   interface specification

Example prompt:

> "I want to create a backend that stores artifacts as encrypted files in a
> `secrets/` directory using age encryption. Using the BACKEND_GUIDE.md
> specification, generate unified check and serialize scripts that work for all
> target types. Each secret should be encrypted with the target's age public key
> stored in the target's config in targets.json."

The AI can help you generate correct script implementations, handle edge cases,
and ensure proper error handling while following the interface specification.

---

_This guide is designed to be self-contained and can be copied to other
repositories. For the latest version, refer to the nixos-artifacts repository._
