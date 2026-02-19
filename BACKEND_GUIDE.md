# Backend Developer Guide for nixos-artifacts

**Version:** 1.0  
**Last Updated:** 2026-02-20  
**Status:** Complete reference for backend implementation

This guide provides everything needed to implement a custom serialization backend for nixos-artifacts. It is designed to be copied standalone to other repositories.

## Table of Contents

- [Overview](#overview)
- [Backend Interface](#backend-interface)
- [The Four Scripts](#the-four-scripts)
  - [check_serialization](#check_serialization)
  - [serialize](#serialize)
  - [deserialize](#deserialize)
  - [shared_serialize (optional)](#shared_serialize-optional)
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

**nixos-artifacts** is a framework that unifies handling of artifacts (secrets and generated files) in NixOS flakes through a common abstraction over multiple backends. It separates the concerns of:

- **Generation**: Creating secret files (SSH keys, SSL certificates, passwords)
- **Serialization**: Storing those files securely (encryption, cloud services, version control)
- **Deserialization**: Retrieving files during system activation

### What is a Backend?

A **backend** is a set of shell scripts that implement the storage contract for artifact files. When you run `artifacts generate`, the CLI orchestrates:

1. **Check Phase**: Calls `check_serialization` to see if the artifact already exists
2. **Generation Phase**: Runs generator scripts to create files (if needed)
3. **Serialization Phase**: Calls your backend's `serialize` script to store the files
4. **Deserialization Phase** (later): Calls `deserialize` during system activation

### Backend Types

Backends must handle two types of targets:

| Type | Description | Permissions |
|------|-------------|-------------|
| **NixOS Machines** | Full system configurations | Has `owner`, `group`, and `mode` |
| **Home Manager Users** | User-level configurations | Only `path` (no system permissions) |
| **Shared Artifacts** | Identical across multiple targets | Stored once, used by many |

### When to Write a Custom Backend

Consider writing a custom backend when:

- You want to integrate with a secret management system not yet supported
- Your organization has specific compliance or storage requirements
- You need custom encryption or access control mechanisms
- You want to store artifacts in a proprietary or internal system

## Backend Interface

The CLI calls your backend scripts at specific points in the artifact lifecycle. All scripts:

- Run in a bubblewrap container with restricted filesystem access
- Receive data through environment variables and temporary files
- Must return exit code 0 on success, non-zero on failure
- Can read configuration from a JSON file (`$config`)

### Script Lifecycle

```
check_serialization
    ↓ (if artifact doesn't exist)
generator runs → creates files in $out
    ↓
serialize stores files from $out
    ↓ (during system activation)
deserialize restores files to $out
```

## The Four Scripts

### check_serialization

**Purpose:** Determine whether an artifact already exists in your backend's storage. This prevents accidental overwrites of existing secrets.

**When Called:** Before the generator script runs, during the `artifacts generate` workflow.

**Environment Variables:**

| Variable | Type | Description | Example |
|----------|------|-------------|---------|
| `$inputs` | Directory | Path to directory containing JSON files with file metadata | `/tmp/artifacts-inputs-xxx` |
| `$config` | File | Path to JSON file with backend configuration | `/tmp/config-xxx.json` |
| `$machine` | String | Target machine name | `server-one` |
| `$artifact` | String | Artifact name being processed | `ssh-host-key` |

**Input Details:**

The `$inputs` directory contains one JSON file per file defined in the artifact's `files` attribute. Each JSON file has this structure:

```json
{
  "path": "/etc/ssh/ssh_host_ed25519_key",
  "owner": "root",
  "group": "root"
}
```

For Home Manager artifacts, `owner` and `group` are omitted (only `path` is present).

**Exit Codes:**

| Exit Code | Meaning | Action Taken by CLI |
|-----------|---------|---------------------|
| 0 | Artifact exists | Skip generation for this artifact |
| Non-zero | Artifact needs generation | Continue to generator phase |

**Output Convention:**

While not required by the CLI, it's conventional to print `EXISTS` to stdout when the artifact is found. This aids in debugging.

**Complete Example:**

```bash
#!/usr/bin/env bash
set -euo pipefail

# Environment variables available:
# $inputs - Directory with expected file metadata
# $config - JSON file with backend configuration
# $machine - Target machine name
# $artifact - Artifact name

# Read storage path from config (with default)
STORAGE_PATH=$(jq -r '.storage_path // "/var/lib/mybackend"' "$config")

# Check if artifact exists in storage
if [[ -f "$STORAGE_PATH/$artifact.json" ]]; then
    echo "EXISTS"
    exit 0
fi

# Artifact doesn't exist - signal that generation is needed
exit 1
```

### serialize

**Purpose:** Store the files generated by the generator script in your backend's storage.

**When Called:** After the generator script successfully completes, for each target that needs the artifact.

**Environment Variables:**

| Variable | Type | Description | Example |
|----------|------|-------------|---------|
| `$out` | Directory | Path to directory containing all generated files | `/tmp/artifacts-out-xxx` |
| `$config` | File | Path to JSON file with backend configuration | `/tmp/config-xxx.json` |
| `$machine` | String | Target machine name or user@host | `server-one` or `alice@workstation` |
| `$artifact` | String | Artifact name being processed | `api-key` |

**Input Details:**

The `$out` directory contains the files created by the generator script. Filenames match the keys in the `files` attribute of your artifact definition.

For example, if your artifact defines:

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
- Use the backend configuration from `$config` for storage parameters
- Return exit code 0 on success, non-zero on failure
- The CLI aborts if this script fails

**Complete Example:**

```bash
#!/usr/bin/env bash
set -euo pipefail

# Environment variables available:
# $out - Directory containing generated files
# $config - JSON file with backend configuration
# $machine - Target machine name
# $artifact - Artifact name

# Read storage path from config
STORAGE_PATH=$(jq -r '.storage_path // "/var/lib/mybackend"' "$config")

# Create storage directory if it doesn't exist
mkdir -p "$STORAGE_PATH"

# Store each generated file
for file in "$out"/*; do
    if [[ -f "$file" ]]; then
        filename=$(basename "$file")
        # Example: copy to storage (replace with your storage logic)
        cp "$file" "$STORAGE_PATH/$artifact-$filename"
        echo "Stored: $artifact-$filename"
    fi
done
```

### deserialize

**Purpose:** Restore stored artifact files for use during system activation.

**When Called:** During NixOS system activation or Home Manager activation.

**Environment Variables:**

| Variable | Type | Description | Example |
|----------|------|-------------|---------|
| `$inputs` | Directory | Path to directory with expected file metadata (same format as check_serialization) | `/tmp/artifacts-inputs-xxx` |
| `$config` | File | Path to JSON file with backend configuration | `/tmp/config-xxx.json` |
| `$machine` | String | Target machine name | `server-one` |
| `$artifact` | String | Artifact name being processed | `database-password` |
| `$out` | Directory | Path where restored files should be placed (you must create this directory) | N/A |

**Output Requirements:**

Your script must create files in the `$out` directory (which doesn't exist yet - you must create it). The CLI will then copy these files to their final destinations with correct ownership and permissions.

**Expected Behavior:**

- Create the `$out` directory
- Restore all files expected by the artifact
- Place restored files in `$out` (filenames must match artifact `files` keys)
- Return exit code 0 on success

**Complete Example:**

```bash
#!/usr/bin/env bash
set -euo pipefail

# Environment variables available:
# $inputs - Directory with expected file metadata
# $config - JSON file with backend configuration
# $machine - Target machine name
# $artifact - Artifact name
# $out - Output directory (must be created)

# Read storage path from config
STORAGE_PATH=$(jq -r '.storage_path // "/var/lib/mybackend"' "$config")

# Create output directory
mkdir -p "$out"

# Restore each expected file
for metadata_file in "$inputs"/*; do
    if [[ -f "$metadata_file" ]]; then
        filename=$(basename "$metadata_file")
        source_file="$STORAGE_PATH/$artifact-$filename"

        if [[ -f "$source_file" ]]; then
            cp "$source_file" "$out/$filename"
            echo "Restored: $filename"
        else
            echo "Error: Missing stored file for $filename" >&2
            exit 1
        fi
    fi
done
```

### shared_serialize (optional)

**Purpose:** Store an artifact for all targets that share it, called once instead of per-target.

**When Called:** After successful generator execution for artifacts with `shared = true`.

**Environment Variables:**

| Variable | Type | Description | Example |
|----------|------|-------------|---------|
| `$out` | Directory | Path to directory containing generated files | `/tmp/artifacts-out-xxx` |
| `$artifact` | String | Artifact name being processed | `wireguard-key` |
| `$machines` | File | Path to JSON file mapping machine names to configs | `/tmp/machines-xxx.json` |
| `$users` | File | Path to JSON file mapping user@host to configs | `/tmp/users-xxx.json` |

**Note:** Unlike other scripts, `$config` is **not** directly available. Read per-target configuration from the `$machines` and `$users` JSON files.

**Input Details:**

The `$machines` JSON file maps machine names to their backend configurations:

```json
{
  "server-one": {
    "storage_path": "/var/lib/mybackend",
    "encryption_key": "abc123"
  },
  "server-two": {
    "storage_path": "/var/lib/mybackend",
    "encryption_key": "def456"
  }
}
```

The `$users` JSON file has the same structure but for Home Manager targets:

```json
{
  "alice@workstation": {
    "storage_path": "~/.local/share/mybackend"
  },
  "bob@laptop": {
    "storage_path": "~/.local/share/mybackend"
  }
}
```

**Expected Behavior:**

- Store the artifact for all machines listed in `$machines`
- Store the artifact for all users listed in `$users`
- Handle the case where one or both files may be empty (no targets of that type)
- Return exit code 0 on success

**Complete Example:**

```bash
#!/usr/bin/env bash
set -euo pipefail

# Environment variables available:
# $artifact - Artifact name
# $out - Directory with generated files
# $machines - JSON file with machine configs
# $users - JSON file with user configs

# Function to store files for a target
store_files() {
    local target="$1"
    local config="$2"
    local storage_path=$(echo "$config" | jq -r '.storage_path // "/var/lib/mybackend"')
    
    mkdir -p "$storage_path"
    
    for file in "$out"/*; do
        if [[ -f "$file" ]]; then
            local filename=$(basename "$file")
            cp "$file" "$storage_path/$artifact-$filename"
            echo "Stored for $target: $artifact-$filename"
        fi
    done
}

# Iterate over machines
if [[ -f "$machines" ]] && [[ "$(jq 'length' "$machines")" -gt 0 ]]; then
    for machine in $(jq -r 'keys[]' "$machines"); do
        machine_config=$(jq -r ".\"$machine\"" "$machines")
        store_files "$machine" "$machine_config"
    done
fi

# Iterate over users
if [[ -f "$users" ]] && [[ "$(jq 'length' "$users")" -gt 0 ]]; then
    for user in $(jq -r 'keys[]' "$users"); do
        user_config=$(jq -r ".\"$user\"" "$users")
        store_files "$user" "$user_config"
    done
fi
```

## Backend Configuration (backend.toml)

The `backend.toml` file defines your backend's scripts and capabilities.

### Complete TOML Structure

```toml
[mybackend]
# NixOS-specific scripts (at least one of nixos_* or home_* is required)
nixos_check_serialization = "./scripts/nixos-check.sh"
nixos_serialize = "./scripts/nixos-serialize.sh"

# Home Manager-specific scripts
home_check_serialization = "./scripts/home-check.sh"
home_serialize = "./scripts/home-serialize.sh"

# Shared artifact scripts (required if capabilities.shared = true)
shared_check_serialization = "./scripts/shared-check.sh"
shared_serialize = "./scripts/shared-serialize.sh"

# Common scripts (used for both NixOS and Home Manager)
deserialize = "./scripts/deserialize.sh"
check_configuration = "./scripts/check-config.sh"

[mybackend.capabilities]
# Whether this backend supports shared artifacts
shared = true

# Whether this backend actually serializes (vs being a test/mock)
serializes = true

[mybackend.settings]
# Default settings available in $config JSON
storage_path = "/var/lib/mybackend"
encryption = "aes256-gcm"
```

### Target-Specific Scripts

You can define different scripts for NixOS and Home Manager targets:

```toml
[mybackend]
nixos_check_serialization = "./nixos-check.sh"  # Has owner/group
home_check_serialization = "./home-check.sh"    # No owner/group
```

If you define both, the CLI calls the appropriate one based on the target type. If you only define one type (e.g., only `nixos_*`), that backend only works with NixOS configurations.

### Capabilities Section

| Capability | Default | Description |
|------------|---------|-------------|
| `shared` | `false` | Set to `true` if your backend supports `shared_serialize`. Required for shared artifacts. |
| `serializes` | `true` | Set to `false` for test backends that don't actually store anything. |

### Include Directive

Backend configurations can be split across multiple files:

```toml
# backend.toml
include = ["./backends/agenix.toml", "./backends/sops.toml"]

[my-custom]
check_serialization = "./check.sh"
serialize = "./serialize.sh"
deserialize = "./deserialize.sh"
```

Paths in `include` are resolved relative to the file containing the directive. Nested includes are supported, and circular includes are detected and rejected.

## Environment Variables Reference

This table summarizes all environment variables available to backend scripts:

| Variable | Available In | Type | Description | Example |
|----------|--------------|------|-------------|---------|
| `$out` | serialize, shared_serialize | Directory | Contains generated files from the generator script | `/tmp/artifacts-out-abc123` |
| `$inputs` | check_serialization, deserialize | Directory | Contains JSON files with file metadata (path, owner, group) | `/tmp/artifacts-inputs-def456` |
| `$config` | check_serialization, serialize, deserialize, check_configuration | File | JSON file with backend settings from `[backend.settings]` and per-target config | `{"storage_path": "/var/lib/secrets"}` |
| `$machine` | check_serialization, serialize, deserialize, check_configuration | String | Target identifier - either NixOS machine name or Home Manager user@host | `server-one` or `alice@workstation` |
| `$artifact` | All scripts | String | The artifact name being processed | `ssh-host-key` |
| `$machines` | shared_serialize | File | JSON file mapping machine names to their `artifacts.config.<backend>.nixos.<machine>` configs | `{"server-one": {"key": "abc"}}` |
| `$users` | shared_serialize | File | JSON file mapping user@host to their `artifacts.config.<backend>.home.<user>` configs | `{"alice@host": {"key": "def"}}` |

## File Format Reference

### $inputs Directory

Each file in `$inputs` is a JSON object with the following structure:

**For NixOS artifacts:**

```json
{
  "path": "/etc/ssh/ssh_host_ed25519_key",
  "owner": "root",
  "group": "root",
  "mode": "600"
}
```

**For Home Manager artifacts:**

```json
{
  "path": "~/.ssh/id_ed25519"
}
```

Note that Home Manager artifacts don't have `owner`, `group`, or `mode` since home-manager doesn't manage system-level permissions.

### $machines JSON

```json
{
  "server-one": {
    "keyFile": "/path/to/key",
    "storagePath": "/var/lib/secrets"
  },
  "server-two": {
    "keyFile": "/path/to/key2",
    "storagePath": "/var/lib/secrets"
  }
}
```

### $users JSON

```json
{
  "alice@workstation": {
    "identityFile": "~/.age/alice.txt",
    "storagePath": "~/.local/share/secrets"
  },
  "bob@laptop": {
    "identityFile": "~/.age/bob.txt",
    "storagePath": "~/.local/share/secrets"
  }
}
```

## Complete Working Example

Here's a minimal but complete backend example that stores artifacts as tar.gz archives.

### Directory Structure

```
my-backend/
├── backend.toml
├── check.sh
├── serialize.sh
└── deserialize.sh
```

### backend.toml

```toml
[my-backend]
nixos_check_serialization = "./check.sh"
nixos_serialize = "./serialize.sh"
home_check_serialization = "./check.sh"
home_serialize = "./serialize.sh"

[my-backend.capabilities]
shared = false
```

### check.sh

```bash
#!/usr/bin/env bash
set -euo pipefail

# Simple check: does file exist?
STORAGE_DIR="${STORAGE_DIR:-./storage}"
ARTIFACT_FILE="$STORAGE_DIR/$artifact.tar.gz"

if [[ -f "$ARTIFACT_FILE" ]]; then
    echo "EXISTS"
    exit 0
else
    exit 1
fi
```

### serialize.sh

```bash
#!/usr/bin/env bash
set -euo pipefail

STORAGE_DIR="${STORAGE_DIR:-./storage}"
mkdir -p "$STORAGE_DIR"

# Archive all files from $out
tar -czf "$STORAGE_DIR/$artifact.tar.gz" -C "$out" .
echo "Serialized: $artifact.tar.gz"
```

### deserialize.sh

```bash
#!/usr/bin/env bash
set -euo pipefail

STORAGE_DIR="${STORAGE_DIR:-./storage}"
mkdir -p "$out"

# Extract to $out
tar -xzf "$STORAGE_DIR/$artifact.tar.gz" -C "$out"
echo "Deserialized: $artifact"
```

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
if [[ ! -f "$config" ]]; then
    echo "Error: Config file not found: $config" >&2
    exit 1
fi
```

### Exit Code Conventions

While the CLI only checks zero vs non-zero, follow these conventions:

| Exit Code | Meaning |
|-----------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Invalid arguments or configuration |
| 3 | Missing dependencies |
| 127 | Command not found |

## Testing

### Testing Phases

1. **Single Artifact Test:**
   Define a simple artifact with one file, test generation and deployment.

2. **Shared Artifact Test:**
   Define a shared artifact across multiple machines or users.

3. **Mixed Target Test:**
   Test with both NixOS and Home Manager configurations.

### Debugging Tips

- Add `echo` statements to your scripts - they're captured in the TUI's log view
- Check the generated JSON files in temporary directories
- Use `set -x` for verbose trace output during development
- Test scripts manually with mocked environment variables

### Test with the CLI

```bash
# List all artifacts
artifacts list backend.toml

# Generate a specific artifact
artifacts tui backend.toml
# Then select your artifact and press Enter
```

## Troubleshooting

### "Backend lacks shared_serialize support"

**Cause:** You tried to generate a shared artifact with a backend that doesn't have `shared_serialize` defined.

**Solution:** Either:
1. Add `shared_serialize` to your backend.toml
2. Set `capabilities.shared = false` if you don't need shared artifacts
3. Use a different backend for shared artifacts

### Exit Code Confusion

**Problem:** Your script returns 0 but the CLI thinks it failed (or vice versa).

**Cause:** The last command in your script determines the exit code.

**Solution:** Explicitly use `exit 0` or `exit 1` at the end of your script.

### Path Issues with $out and $inputs

**Problem:** Files aren't being found in `$out` or `$inputs`.

**Cause:** These directories contain files, not subdirectories. Iterate with `for file in "$out"/*`.

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

**Cause:** The `$config` file might be empty or malformed.

**Solution:** Use the `//` operator to provide defaults:
```bash
STORAGE_PATH=$(jq -r '.storage_path // "/default/path"' "$config")
```

### Permission Issues

**Problem:** Your script can't write to the storage directory.

**Cause:** The bubblewrap container restricts filesystem access.

**Solution:** Ensure your storage path is accessible within the container's bind mounts. Use paths under `/var/lib`, `/tmp`, or paths explicitly mounted by the CLI.

## See Also

- **Full Documentation:** See the Antora documentation site for comprehensive guides
- **Example Backends:** Check `pkgs/artifacts/examples/backends/` in the nixos-artifacts repository
- **CLI Reference:** Run `artifacts --help` for command-line options
- **Repository:** https://github.com/mrVanDalo/nixos-artifacts

---

*This guide is designed to be self-contained and can be copied to other repositories. For the latest version, refer to the nixos-artifacts repository.*
