# Backend Developer Guide for nixos-artifacts

**Version:** 2.0\
**Last Updated:** 2026-03-11\
**Status:** Complete reference for backend implementation

This guide provides everything needed to implement a custom serialization
backend for nixos-artifacts. It is designed to be copied standalone to other
repositories.

## Table of Contents

- [Overview](#overview)
- [Backend Interface](#backend-interface)
- [Backend Scripts](#backend-scripts)
  - [check_serialization](#check_serialization)
    - [nixos_check_serialization](#nixos_check_serialization)
    - [home_check_serialization](#home_check_serialization)
    - [shared_check_serialization](#shared_check_serialization)
  - [serialize](#serialize)
    - [nixos_serialize](#nixos_serialize)
    - [home_serialize](#home_serialize)
    - [shared_serialize](#shared_serialize)
  - [deserialize](#deserialize)
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

1. **Check Phase**: Calls `check_serialization` to see if the artifact already
   exists
2. **Generation Phase**: Runs generator scripts to create files (if needed)
3. **Serialization Phase**: Calls your backend's `serialize` script to store the
   files
4. **Deserialization Phase** (later): Calls `deserialize` during system
   activation

### Backend Types

Backends handle three types of targets:

| Type                   | Description                              | Permissions                           |
| ---------------------- | ---------------------------------------- | ------------------------------------- |
| **NixOS Machines**     | Full system configurations               | Has `owner` and `group` fields        |
| **Home Manager Users** | User-level configurations                | `owner` and `group` are `null`        |
| **Shared Artifacts**   | Identical across multiple machines/users | Requires dedicated `shared_*` scripts |

### When to Write a Custom Backend

Consider writing a custom backend when:

- You want to integrate with a secret management system not yet supported
- Your organization has specific compliance or storage requirements
- You need custom encryption or access control mechanisms
- You want to store artifacts in a proprietary or internal system

## Backend Interface

The CLI calls your backend scripts at specific points in the artifact lifecycle.
All scripts:

- Receive data through environment variables and temporary files
- Must return exit code 0 on success, non-zero on failure
- Can read configuration from a JSON file (`$config`)

Note: Generator scripts run in isolated bubblewrap containers for security, but
backend serialization scripts run directly on the host system.

### Script Lifecycle

```
check_serialization
    ↓ (if artifact doesn't exist)
generator runs → creates files in $out
    ↓
serialize stores files from $out
```

For shared artifacts, the lifecycle uses `shared_check_serialization` and
`shared_serialize` instead.

## Backend Scripts

### check_serialization

**Purpose:** Determine whether an artifact already exists in your backend's
storage. This prevents accidental overwrites of existing secrets.

**When Called:** Before the generator script runs, during the
`artifacts generate` workflow.

**Exit Codes:**

| Exit Code | Meaning                   | Action Taken by CLI               |
| --------- | ------------------------- | --------------------------------- |
| 0         | Artifact exists           | Skip generation for this artifact |
| Non-zero  | Artifact needs generation | Continue to generator phase       |

#### nixos_check_serialization

Called for NixOS machine targets.

**Environment Variables:**

| Variable            | Type      | Description                                                | Example                     |
| ------------------- | --------- | ---------------------------------------------------------- | --------------------------- |
| `$inputs`           | Directory | Path to directory containing JSON files with file metadata | `/tmp/artifacts-inputs-xxx` |
| `$config`           | File      | Path to JSON file with backend configuration               | `/tmp/config-xxx.json`      |
| `$artifact_context` | String    | Context type: always `"nixos"`                             | `nixos`                     |
| `$machine`          | String    | Target machine name                                        | `server-one`                |
| `$artifact`         | String    | Artifact name being processed                              | `ssh-host-key`              |

**$inputs Directory Format:**

Each file in `$inputs` is named after a file key and contains JSON:

```json
{
  "path": "/etc/ssh/ssh_host_ed25519_key",
  "owner": "root",
  "group": "root"
}
```

**Example:**

```bash
#!/usr/bin/env bash
set -euo pipefail

# Read storage path from config (with default)
STORAGE_PATH=$(jq -r '.storage_path // "/var/lib/mybackend"' "$config")

# Check if artifact exists for this machine
if [[ -f "$STORAGE_PATH/$machine/$artifact.json" ]]; then
    echo "EXISTS"
    exit 0
fi

# Artifact doesn't exist - signal that generation is needed
exit 1
```

#### home_check_serialization

Called for Home Manager user targets.

**Environment Variables:**

| Variable            | Type      | Description                                                | Example                     |
| ------------------- | --------- | ---------------------------------------------------------- | --------------------------- |
| `$inputs`           | Directory | Path to directory containing JSON files with file metadata | `/tmp/artifacts-inputs-xxx` |
| `$config`           | File      | Path to JSON file with backend configuration               | `/tmp/config-xxx.json`      |
| `$artifact_context` | String    | Context type: always `"homemanager"`                       | `homemanager`               |
| `$username`         | String    | Target user identifier                                     | `alice@workstation`         |
| `$artifact`         | String    | Artifact name being processed                              | `ssh-host-key`              |

**$inputs Directory Format:**

Each file in `$inputs` contains JSON where `owner` and `group` may be `null`:

```json
{
  "path": "~/.ssh/id_ed25519",
  "owner": null,
  "group": null
}
```

**Example:**

```bash
#!/usr/bin/env bash
set -euo pipefail

# Read storage path from config (with default)
STORAGE_PATH=$(jq -r '.storage_path // "/var/lib/mybackend"' "$config")

# Check if artifact exists for this user
if [[ -f "$STORAGE_PATH/$username/$artifact.json" ]]; then
    echo "EXISTS"
    exit 0
fi

# Artifact doesn't exist - signal that generation is needed
exit 1
```

#### shared_check_serialization

Called for shared artifacts (required if `capabilities.shared = true`). Must
check if the artifact exists for ALL targets that share it.

**Environment Variables:**

| Variable    | Type   | Description                                      | Example                  |
| ----------- | ------ | ------------------------------------------------ | ------------------------ |
| `$artifact` | String | Artifact name being processed                    | `wireguard-key`          |
| `$machines` | File   | JSON file mapping machine names to their configs | `/tmp/machines-xxx.json` |
| `$users`    | File   | JSON file mapping user@host to their configs     | `/tmp/users-xxx.json`    |

**$machines JSON Format:**

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

**$users JSON Format:**

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

**Example:**

```bash
#!/usr/bin/env bash
set -euo pipefail

# Check if artifact exists for ALL targets
STORAGE_PATH="/var/lib/mybackend"

# Helper to check if artifact exists for a target
check_target() {
    local target="$1"
    [[ -f "$STORAGE_PATH/$target/$artifact.json" ]]
}

# Check all machines
if [[ -f "$machines" ]]; then
    while read -r machine; do
        if ! check_target "$machine"; then
            exit 1  # Missing for this machine
        fi
    done < <(jq -r 'keys[]' "$machines")
fi

# Check all users  
if [[ -f "$users" ]]; then
    while read -r user; do
        if ! check_target "$user"; then
            exit 1  # Missing for this user
        fi
    done < <(jq -r 'keys[]' "$users")
fi

# All targets have the artifact
exit 0
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

#### nixos_serialize

Called for NixOS machine targets.

**Environment Variables:**

| Variable            | Type      | Description                                      | Example                  |
| ------------------- | --------- | ------------------------------------------------ | ------------------------ |
| `$out`              | Directory | Path to directory containing all generated files | `/tmp/artifacts-out-xxx` |
| `$config`           | File      | Path to JSON file with backend configuration     | `/tmp/config-xxx.json`   |
| `$artifact_context` | String    | Context type: always `"nixos"`                   | `nixos`                  |
| `$machine`          | String    | Target machine name                              | `server-one`             |
| `$artifact`         | String    | Artifact name being processed                    | `api-key`                |

**Example:**

```bash
#!/usr/bin/env bash
set -euo pipefail

# Read storage path from config
STORAGE_PATH=$(jq -r '.storage_path // "/var/lib/mybackend"' "$config")

# Create storage directory if it doesn't exist
mkdir -p "$STORAGE_PATH/$machine"

# Store each generated file
for file in "$out"/*; do
    if [[ -f "$file" ]]; then
        filename=$(basename "$file")
        cp "$file" "$STORAGE_PATH/$machine/$artifact-$filename"
        echo "Stored: $machine/$artifact-$filename"
    fi
done
```

#### home_serialize

Called for Home Manager user targets.

**Environment Variables:**

| Variable            | Type      | Description                                      | Example                  |
| ------------------- | --------- | ------------------------------------------------ | ------------------------ |
| `$out`              | Directory | Path to directory containing all generated files | `/tmp/artifacts-out-xxx` |
| `$config`           | File      | Path to JSON file with backend configuration     | `/tmp/config-xxx.json`   |
| `$artifact_context` | String    | Context type: always `"homemanager"`             | `homemanager`            |
| `$username`         | String    | Target user identifier                           | `alice@workstation`      |
| `$artifact`         | String    | Artifact name being processed                    | `api-key`                |

**Example:**

```bash
#!/usr/bin/env bash
set -euo pipefail

# Read storage path from config
STORAGE_PATH=$(jq -r '.storage_path // "/var/lib/mybackend"' "$config")

# Create storage directory if it doesn't exist
mkdir -p "$STORAGE_PATH/$username"

# Store each generated file
for file in "$out"/*; do
    if [[ -f "$file" ]]; then
        filename=$(basename "$file")
        cp "$file" "$STORAGE_PATH/$username/$artifact-$filename"
        echo "Stored: $username/$artifact-$filename"
    fi
done
```

#### shared_serialize

Called for shared artifacts (required if `capabilities.shared = true`). Must
store the artifact for ALL targets that share it.

**Environment Variables:**

| Variable    | Type      | Description                                      | Example                  |
| ----------- | --------- | ------------------------------------------------ | ------------------------ |
| `$out`      | Directory | Path to directory containing all generated files | `/tmp/artifacts-out-xxx` |
| `$artifact` | String    | Artifact name being processed                    | `wireguard-key`          |
| `$machines` | File      | JSON file mapping machine names to their configs | `/tmp/machines-xxx.json` |
| `$users`    | File      | JSON file mapping user@host to their configs     | `/tmp/users-xxx.json`    |

Note: Unlike per-target scripts, `$config` is not available. Read per-target
configuration from `$machines` and `$users`.

**Expected Behavior:**

- Store the artifact for all machines listed in `$machines`
- Store the artifact for all users listed in `$users`
- Handle the case where one or both files may be empty (no targets of that type)
- Return exit code 0 on success

**Example:**

```bash
#!/usr/bin/env bash
set -euo pipefail

# Function to store files for a target
store_files() {
    local target="$1"
    local config="$2"
    local storage_path=$(echo "$config" | jq -r '.storage_path // "/var/lib/mybackend"')
    
    mkdir -p "$storage_path/$target"
    
    for file in "$out"/*; do
        if [[ -f "$file" ]]; then
            local filename=$(basename "$file")
            cp "$file" "$storage_path/$target/$artifact-$filename"
            echo "Stored for $target: $artifact-$filename"
        fi
    done
}

# Store for all machines
if [[ -f "$machines" ]] && [[ "$(jq 'length' "$machines")" -gt 0 ]]; then
    for machine in $(jq -r 'keys[]' "$machines"); do
        machine_config=$(jq -c ".\"$machine\"" "$machines")
        store_files "$machine" "$machine_config"
    done
fi

# Store for all users
if [[ -f "$users" ]] && [[ "$(jq 'length' "$users")" -gt 0 ]]; then
    for user in $(jq -r 'keys[]' "$users"); do
        user_config=$(jq -c ".\"$user\"" "$users")
        store_files "$user" "$user_config"
    done
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

#### nixos_serialize

Called for NixOS machine targets.

**Environment Variables:**

| Variable            | Type      | Description                                      | Example                  |
| ------------------- | --------- | ------------------------------------------------ | ------------------------ |
| `$out`              | Directory | Path to directory containing all generated files | `/tmp/artifacts-out-xxx` |
| `$config`           | File      | Path to JSON file with backend configuration     | `/tmp/config-xxx.json`   |
| `$artifact_context` | String    | Context type: always `"nixos"`                   | `nixos`                  |
| `$machine`          | String    | Target machine name                              | `server-one`             |
| `$artifact`         | String    | Artifact name being processed                    | `api-key`                |

**Example:**

```bash
#!/usr/bin/env bash
set -euo pipefail

# Read storage path from config
STORAGE_PATH=$(jq -r '.storage_path // "/var/lib/mybackend"' "$config")

# Create storage directory if it doesn't exist
mkdir -p "$STORAGE_PATH/$machine"

# Store each generated file
for file in "$out"/*; do
    if [[ -f "$file" ]]; then
        filename=$(basename "$file")
        cp "$file" "$STORAGE_PATH/$machine/$artifact-$filename"
        echo "Stored: $machine/$artifact-$filename"
    fi
done
```

#### home_serialize

Called for Home Manager user targets.

**Environment Variables:**

| Variable            | Type      | Description                                      | Example                  |
| ------------------- | --------- | ------------------------------------------------ | ------------------------ |
| `$out`              | Directory | Path to directory containing all generated files | `/tmp/artifacts-out-xxx` |
| `$config`           | File      | Path to JSON file with backend configuration     | `/tmp/config-xxx.json`   |
| `$artifact_context` | String    | Context type: always `"homemanager"`             | `homemanager`            |
| `$username`         | String    | Target user identifier                           | `alice@workstation`      |
| `$artifact`         | String    | Artifact name being processed                    | `api-key`                |

**Example:**

```bash
#!/usr/bin/env bash
set -euo pipefail

# Read storage path from config
STORAGE_PATH=$(jq -r '.storage_path // "/var/lib/mybackend"' "$config")

# Create storage directory if it doesn't exist
mkdir -p "$STORAGE_PATH/$username"

# Store each generated file
for file in "$out"/*; do
    if [[ -f "$file" ]]; then
        filename=$(basename "$file")
        cp "$file" "$STORAGE_PATH/$username/$artifact-$filename"
        echo "Stored: $username/$artifact-$filename"
    fi
done
```

#### shared_serialize

Called for shared artifacts (required if `capabilities.shared = true`). Must
store the artifact for ALL targets that share it.

**Environment Variables:**

| Variable    | Type      | Description                                      | Example                  |
| ----------- | --------- | ------------------------------------------------ | ------------------------ |
| `$out`      | Directory | Path to directory containing all generated files | `/tmp/artifacts-out-xxx` |
| `$artifact` | String    | Artifact name being processed                    | `wireguard-key`          |
| `$machines` | File      | JSON file mapping machine names to their configs | `/tmp/machines-xxx.json` |
| `$users`    | File      | JSON file mapping user@host to their configs     | `/tmp/users-xxx.json`    |

Note: Unlike per-target scripts, `$config` is not available. Read per-target
configuration from `$machines` and `$users`.

**$machines JSON Format:**

```json
{
  "server-one": {
    "storage_path": "/var/lib/mybackend",
    "encryption_key": "abc123"
  }
}
```

**$users JSON Format:**

```json
{
  "alice@workstation": {
    "storage_path": "~/.local/share/mybackend"
  }
}
```

**Expected Behavior:**

- Store the artifact for all machines listed in `$machines`
- Store the artifact for all users listed in `$users`
- Handle the case where one or both files may be empty (no targets of that type)
- Return exit code 0 on success

**Example:**

```bash
#!/usr/bin/env bash
set -euo pipefail

# Function to store files for a target
store_files() {
    local target="$1"
    local config="$2"
    local storage_path=$(echo "$config" | jq -r '.storage_path // "/var/lib/mybackend"')
    
    mkdir -p "$storage_path/$target"
    
    for file in "$out"/*; do
        if [[ -f "$file" ]]; then
            local filename=$(basename "$file")
            cp "$file" "$storage_path/$target/$artifact-$filename"
            echo "Stored for $target: $artifact-$filename"
        fi
    done
}

# Store for all machines
if [[ -f "$machines" ]] && [[ "$(jq 'length' "$machines")" -gt 0 ]]; then
    for machine in $(jq -r 'keys[]' "$machines"); do
        machine_config=$(jq -c ".\"$machine\"" "$machines")
        store_files "$machine" "$machine_config"
    done
fi

# Store for all users
if [[ -f "$users" ]] && [[ "$(jq 'length' "$users")" -gt 0 ]]; then
    for user in $(jq -r 'keys[]' "$users"); do
        user_config=$(jq -c ".\"$user\"" "$users")
        store_files "$user" "$user_config"
    done
fi
```

### deserialize

**Note:** The `deserialize` script is defined as part of the backend interface
but is not currently used by the artifacts CLI. Deserialization happens during
NixOS system activation via NixOS module integration, which is managed
separately by backend implementations (e.g., agenix, sops-nix).

If you're implementing a backend, you may define a `deserialize` script for
documentation purposes, but it won't be called by this CLI tool.

## Backend Configuration (backend.toml)

The `backend.toml` file defines your backend's scripts and capabilities.

### Complete TOML Structure

```toml
[mybackend]
# NixOS-specific scripts (required if serializes = true, default)
nixos_check_serialization = "./scripts/nixos-check.sh"
nixos_serialize = "./scripts/nixos-serialize.sh"

# Home Manager-specific scripts (required if serializes = true, default)
home_check_serialization = "./scripts/home-check.sh"
home_serialize = "./scripts/home-serialize.sh"

# Shared artifact scripts (required if capabilities.shared = true)
shared_check_serialization = "./scripts/shared-check.sh"
shared_serialize = "./scripts/shared-serialize.sh"

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
nixos_check_serialization = "./nixos-check.sh"  # Has $machine env var
home_check_serialization = "./home-check.sh"    # Has $username env var
```

If you define both, the CLI calls the appropriate one based on the target type.
If you only define one type (e.g., only `nixos_*`), that backend only works with
NixOS configurations.

### Capabilities Section

| Capability   | Default | Description                                                                               |
| ------------ | ------- | ----------------------------------------------------------------------------------------- |
| `shared`     | `false` | Set to `true` if your backend supports `shared_serialize`. Required for shared artifacts. |
| `serializes` | `true`  | Set to `false` for test backends that don't actually store anything.                      |

When `serializes = false`, no scripts are required. This is useful for test/mock
backends.

### Include Directive

Backend configurations can be split across multiple files:

```toml
# backend.toml
include = ["./backends/agenix.toml", "./backends/sops.toml"]

[my-custom]
nixos_check_serialization = "./check.sh"
nixos_serialize = "./serialize.sh"
home_check_serialization = "./check.sh"
home_serialize = "./serialize.sh"
```

Paths in `include` are resolved relative to the file containing the directive.
Nested includes are supported, and circular includes are detected and rejected.

## Environment Variables Reference

### Environment Variables by Script Type

| Variable            | nixos_check / nixos_serialize | home_check / home_serialize | shared_check | shared_serialize |
| ------------------- | ----------------------------- | --------------------------- | ------------ | ---------------- |
| `$out`              | ✅ (serialize only)           | ✅ (serialize only)         | ❌           | ✅               |
| `$inputs`           | ✅ (check only)               | ✅ (check only)             | ❌           | ❌               |
| `$config`           | ✅                            | ✅                          | ❌           | ❌               |
| `$artifact_context` | ✅                            | ✅                          | ❌           | ❌               |
| `$machine`          | ✅ (NixOS only)               | ❌                          | ❌           | ❌               |
| `$username`         | ❌                            | ✅ (Home only)              | ❌           | ❌               |
| `$artifact`         | ✅                            | ✅                          | ✅           | ✅               |
| `$machines`         | ❌                            | ❌                          | ✅           | ✅               |
| `$users`            | ❌                            | ❌                          | ✅           | ✅               |

### Variable Details

| Variable            | Type      | Description                                               | Example                                |
| ------------------- | --------- | --------------------------------------------------------- | -------------------------------------- |
| `$out`              | Directory | Contains generated files from the generator script        | `/tmp/artifacts-out-abc123`            |
| `$inputs`           | Directory | Contains JSON files with file metadata                    | `/tmp/artifacts-inputs-def456`         |
| `$config`           | File      | JSON file with backend settings from `[backend.settings]` | `{"storage_path": "/var/lib/secrets"}` |
| `$artifact_context` | String    | Context type: `"nixos"` or `"homemanager"`                | `nixos`                                |
| `$machine`          | String    | NixOS machine name (NixOS scripts only)                   | `server-one`                           |
| `$username`         | String    | Home Manager user identifier (Home scripts only)          | `alice@workstation`                    |
| `$artifact`         | String    | The artifact name being processed                         | `ssh-host-key`                         |
| `$machines`         | File      | JSON mapping machine names to their backend configs       | `{"server-one": {"key": "abc"}}`       |
| `$users`            | File      | JSON mapping user@host to their backend configs           | `{"alice@host": {"key": "def"}}`       |

## File Format Reference

### $inputs Directory

Each file in `$inputs` is a JSON object with the following structure:

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

Here's a minimal but complete backend example that stores artifacts as tar.gz
archives.

### Directory Structure

```
my-backend/
├── backend.toml
├── check.sh
└── serialize.sh
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

# Environment variables available:
# $inputs - Directory with expected file metadata
# $config - JSON file with backend configuration
# $artifact_context - "nixos" or "homemanager"
# $machine - Target machine name (NixOS only)
# $username - Target user name (Home Manager only)
# $artifact - Artifact name

# Determine target identifier
if [[ "$artifact_context" == "nixos" ]]; then
    TARGET="$machine"
else
    TARGET="$username"
fi

# Read storage path from config (with default)
STORAGE_DIR="${STORAGE_DIR:-./storage}"
ARTIFACT_FILE="$STORAGE_DIR/$TARGET/$artifact.tar.gz"

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

# Environment variables available:
# $out - Directory containing generated files
# $config - JSON file with backend configuration
# $artifact_context - "nixos" or "homemanager"
# $machine - Target machine name (NixOS only)
# $username - Target user name (Home Manager only)
# $artifact - Artifact name

# Determine target identifier
if [[ "$artifact_context" == "nixos" ]]; then
    TARGET="$machine"
else
    TARGET="$username"
fi

# Read storage path from config
STORAGE_DIR="${STORAGE_DIR:-./storage}"
mkdir -p "$STORAGE_DIR/$TARGET"

# Archive all files from $out
tar -czf "$STORAGE_DIR/$TARGET/$artifact.tar.gz" -C "$out" .
echo "Serialized: $TARGET/$artifact.tar.gz"
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
# List all artifacts
artifacts list backend.toml

# Generate a specific artifact
artifacts tui backend.toml
# Then select your artifact and press Enter
```

## Troubleshooting

### "Backend lacks shared_serialize support"

**Cause:** You tried to generate a shared artifact with a backend that doesn't
have `shared_serialize` defined.

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

**Cause:** The `$config` file might be empty or malformed.

**Solution:** Use the `//` operator to provide defaults:

```bash
STORAGE_PATH=$(jq -r '.storage_path // "/default/path"' "$config")
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

---

_This guide is designed to be self-contained and can be copied to other
repositories. For the latest version, refer to the nixos-artifacts repository._
