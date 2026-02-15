# Backend Implementation Guide

This document provides detailed instructions for implementing a backend for
NixOS Artifacts Store. A backend is responsible for serializing, deserializing,
and checking the state of generated artifacts.

## Overview

A backend consists of:

1. **TOML Configuration** (`backend.toml`) - Defines script paths and
   capabilities
2. **Shell Scripts** - Executable scripts for serialization operations
3. **NixOS Module** (optional) - Defines `artifacts.config.<backend>` options

## Backend TOML Configuration

### File Structure

Create a `backend.toml` file with the following structure:

```toml
[backend-name]
check_serialization = "./scripts/check.sh"
serialize = "./scripts/serialize.sh"
deserialize = "./scripts/deserialize.sh"
shared_serialize = "./scripts/shared_serialize.sh"  # Optional

[backend-name.settings]
# Backend-specific settings (passed to NixOS module)
key = "value"
another_key = 123

[backend-name.capabilities]
shared = true       # Optional: supports shared artifacts (inferred from shared_serialize if not set)
serializes = true   # Optional: actually persists secrets (default: true)
```

### Configuration Fields

| Field                     | Type   | Required | Description                                                           |
| ------------------------- | ------ | -------- | --------------------------------------------------------------------- |
| `check_serialization`     | string | Yes*     | Path to script that checks if regeneration is needed                  |
| `serialize`               | string | Yes*     | Path to script that serializes per-machine artifacts                  |
| `deserialize`             | string | Yes*     | Path to script that deserializes artifacts                            |
| `shared_serialize`        | string | No       | Path to script that serializes shared artifacts                       |
| `settings`                | table  | No       | Key-value pairs passed to the backend                                 |
| `capabilities.shared`     | bool   | No       | Supports shared artifacts (inferred from `shared_serialize` presence) |
| `capabilities.serializes` | bool   | No       | Actually persists secrets (default: `true`)                           |

*Required only if `capabilities.serializes = true` (the default).

### Script Path Resolution

- Paths are resolved relative to the `backend.toml` file location
- Absolute paths are used as-is
- Use `./` prefix for relative paths

### Include Directive

Split configuration across multiple files:

```toml
# backend.toml
include = ["./backends/agenix.toml", "./backends/sops.toml"]

[local-backend]
check_serialization = "./local_check.sh"
serialize = "./local_serialize.sh"
deserialize = "./local_deserialize.sh"
```

- Paths in includes are relative to the file containing the `include`
- Nested includes are supported
- Circular includes are detected and rejected
- Duplicate backend names produce an error

---

## Required Scripts

### 1. check_serialization

Determines if an artifact needs regeneration.

**Exit Codes:**

- `0` - Artifact is up-to-date, skip generation
- Non-zero - Artifact needs regeneration

**Environment Variables (per-machine artifacts):**

| Variable            | Description                                                |
| ------------------- | ---------------------------------------------------------- |
| `$inputs`           | Directory containing file metadata JSON files              |
| `$config`           | Path to JSON file with `artifacts.config.<backend>` values |
| `$artifact`         | Artifact name                                              |
| `$artifact_context` | Either `nixos` or `homemanager`                            |
| `$machine`          | Machine name (only when `artifact_context=nixos`)          |
| `$username`         | Username (only when `artifact_context=homemanager`)        |

**Input Files Structure:**

For each file defined in the artifact, a JSON file is created in
`$inputs/<file-name>`:

```json
{
  "path": "/run/secrets/my-secret",
  "owner": "root",
  "group": "root"
}
```

**Example:**

```bash
#!/usr/bin/env bash
set -euo pipefail

# Check if all serialized files exist
for input in "$inputs"/*; do
    filename=$(basename "$input")
    path=$(jq -r '.path' "$input")

    # Check if encrypted version exists
    if [[ ! -f "./secrets/${machine}/${artifact}/${filename}.age" ]]; then
        echo "Missing: ${filename}"
        exit 1
    fi
done

echo "Up to date"
exit 0
```

---

### 2. serialize

Serializes generated artifact files for a single machine/user.

**Exit Codes:**

- `0` - Serialization successful
- Non-zero - Serialization failed (stops generation)

**Environment Variables:**

| Variable            | Description                                                |
| ------------------- | ---------------------------------------------------------- |
| `$out`              | Directory containing generated files to serialize          |
| `$config`           | Path to JSON file with `artifacts.config.<backend>` values |
| `$artifact`         | Artifact name                                              |
| `$artifact_context` | Either `nixos` or `homemanager`                            |
| `$machine`          | Machine name (only when `artifact_context=nixos`)          |
| `$username`         | Username (only when `artifact_context=homemanager`)        |

**Example (agenix-style):**

```bash
#!/usr/bin/env bash
set -euo pipefail

# Read the public key from config
public_key=$(jq -r '.publicKey' "$config")

# Create output directory
mkdir -p "./secrets/${machine}/${artifact}"

# Encrypt each generated file
for file in "$out"/*; do
    if [[ -f "$file" ]]; then
        filename=$(basename "$file")
        age -r "$public_key" -o "./secrets/${machine}/${artifact}/${filename}.age" "$file"
        echo "Encrypted: ${filename}"
    fi
done
```

---

### 3. deserialize

Deserializes (decrypts) stored artifacts. Called during deployment.

**Exit Codes:**

- `0` - Deserialization successful
- Non-zero - Deserialization failed

**Environment Variables:**

| Variable            | Description                                                |
| ------------------- | ---------------------------------------------------------- |
| `$out`              | Directory where decrypted files should be written          |
| `$config`           | Path to JSON file with `artifacts.config.<backend>` values |
| `$artifact`         | Artifact name                                              |
| `$artifact_context` | Either `nixos` or `homemanager`                            |
| `$machine`          | Machine name (only when `artifact_context=nixos`)          |
| `$username`         | Username (only when `artifact_context=homemanager`)        |

**Example:**

```bash
#!/usr/bin/env bash
set -euo pipefail

identity_file=$(jq -r '.identityFile' "$config")

for encrypted in "./secrets/${machine}/${artifact}"/*.age; do
    if [[ -f "$encrypted" ]]; then
        filename=$(basename "$encrypted" .age)
        age -d -i "$identity_file" -o "$out/${filename}" "$encrypted"
    fi
done
```

---

### 4. shared_serialize (Optional)

Serializes shared artifacts that span multiple machines and/or users.

**Required when:** `capabilities.shared = true` and
`capabilities.serializes = true`

**Exit Codes:**

- `0` - Serialization successful
- Non-zero - Serialization failed

**Environment Variables:**

| Variable    | Description                                                      |
| ----------- | ---------------------------------------------------------------- |
| `$artifact` | Artifact name                                                    |
| `$out`      | Directory containing generated files to serialize                |
| `$machines` | Path to JSON file mapping machine names to their backend configs |
| `$users`    | Path to JSON file mapping usernames to their backend configs     |

**machines.json structure:**

```json
{
  "server-1": {
    "publicKey": "age1...",
    "identityFile": "/etc/secrets/identity"
  },
  "server-2": {
    "publicKey": "age1...",
    "identityFile": "/etc/secrets/identity"
  }
}
```

**users.json structure:**

```json
{
  "alice@workstation": {
    "publicKey": "age1...",
    "identityFile": "/home/alice/.age/key"
  }
}
```

**Example:**

```bash
#!/usr/bin/env bash
set -euo pipefail

mkdir -p "./shared-secrets/${artifact}"

# Collect all recipient public keys
recipients=()
for machine in $(jq -r 'keys[]' "$machines"); do
    key=$(jq -r --arg m "$machine" '.[$m].publicKey' "$machines")
    recipients+=("-r" "$key")
done
for user in $(jq -r 'keys[]' "$users"); do
    key=$(jq -r --arg u "$user" '.[$u].publicKey' "$users")
    recipients+=("-r" "$key")
done

# Encrypt each file for all recipients
for file in "$out"/*; do
    if [[ -f "$file" ]]; then
        filename=$(basename "$file")
        age "${recipients[@]}" -o "./shared-secrets/${artifact}/${filename}.age" "$file"
    fi
done
```

---

## NixOS Module Options

Define backend-specific configuration options in a NixOS module.

### Module Structure

```nix
# modules/backends/agenix.nix
{ lib, config, ... }:
with lib;
{
  options.artifacts.config.agenix = {

    publicKey = mkOption {
      type = types.str;
      description = "Age public key for encrypting secrets";
      example = "age1qyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqszqgpqyqs3290gq";
    };

    identityFile = mkOption {
      type = types.path;
      default = "/etc/secrets/identity.txt";
      description = "Path to the age identity file for decryption";
    };

    secretsDir = mkOption {
      type = types.path;
      default = ./secrets;
      description = "Directory where encrypted secrets are stored";
    };

  };
}
```

### Option Naming Convention

- Use `artifacts.config.<backend-name>.<option>` structure
- Keep backend names lowercase and hyphen-separated
- Use descriptive option names

### Per-Machine Configuration

Each machine/user can have different values:

```nix
# Machine 1
{
  artifacts.config.agenix = {
    publicKey = "age1server1...";
    identityFile = "/etc/secrets/server1.txt";
  };
}

# Machine 2
{
  artifacts.config.agenix = {
    publicKey = "age1server2...";
    identityFile = "/etc/secrets/server2.txt";
  };
}
```

### Config JSON Format

Scripts receive these options as JSON in `$config`:

```json
{
  "publicKey": "age1server1...",
  "identityFile": "/etc/secrets/server1.txt",
  "secretsDir": "/nix/store/.../secrets"
}
```

---

## Capability Declarations

### serializes

Controls whether the backend actually persists secrets.

```toml
[test-backend.capabilities]
serializes = false
```

- `true` (default): Requires all scripts, performs actual serialization
- `false`: Scripts are optional, useful for testing or passthrough backends

### shared

Controls whether the backend supports shared artifacts.

```toml
[agenix.capabilities]
shared = true
```

- When `true`: Requires `shared_serialize` script (if `serializes=true`)
- When not set: Inferred from presence of `shared_serialize` script
- When `false`: Shared artifacts using this backend will fail

---

## Generator Script Environment

For reference, generator scripts receive these environment variables:

| Variable            | Description                                     |
| ------------------- | ----------------------------------------------- |
| `$out`              | Directory where generated files must be written |
| `$prompts`          | Directory containing user prompt inputs         |
| `$artifact`         | Artifact name                                   |
| `$artifact_context` | `nixos`, `homemanager`, or `shared`             |
| `$machine`          | Machine name (only for `nixos` context)         |
| `$username`         | Username (only for `homemanager` context)       |

---

## Complete Backend Example

### Directory Structure

```
my-backend/
├── backend.toml
├── scripts/
│   ├── check.sh
│   ├── serialize.sh
│   ├── deserialize.sh
│   └── shared_serialize.sh
└── module.nix
```

### backend.toml

```toml
[my-backend]
check_serialization = "./scripts/check.sh"
serialize = "./scripts/serialize.sh"
deserialize = "./scripts/deserialize.sh"
shared_serialize = "./scripts/shared_serialize.sh"

[my-backend.capabilities]
shared = true
serializes = true
```

### scripts/check.sh

```bash
#!/usr/bin/env bash
set -euo pipefail

secrets_dir=$(jq -r '.secretsDir' "$config")

for input in "$inputs"/*; do
    filename=$(basename "$input")
    target="${secrets_dir}/${machine}/${artifact}/${filename}.enc"

    if [[ ! -f "$target" ]]; then
        echo "Needs generation: $filename"
        exit 1
    fi
done

exit 0
```

### scripts/serialize.sh

```bash
#!/usr/bin/env bash
set -euo pipefail

secrets_dir=$(jq -r '.secretsDir' "$config")
key=$(jq -r '.encryptionKey' "$config")

mkdir -p "${secrets_dir}/${machine}/${artifact}"

for file in "$out"/*; do
    if [[ -f "$file" ]]; then
        filename=$(basename "$file")
        # Your encryption logic here
        cp "$file" "${secrets_dir}/${machine}/${artifact}/${filename}.enc"
    fi
done
```

### scripts/deserialize.sh

```bash
#!/usr/bin/env bash
set -euo pipefail

secrets_dir=$(jq -r '.secretsDir' "$config")
key=$(jq -r '.decryptionKey' "$config")

for encrypted in "${secrets_dir}/${machine}/${artifact}"/*.enc; do
    if [[ -f "$encrypted" ]]; then
        filename=$(basename "$encrypted" .enc)
        # Your decryption logic here
        cp "$encrypted" "$out/${filename}"
    fi
done
```

### scripts/shared_serialize.sh

```bash
#!/usr/bin/env bash
set -euo pipefail

mkdir -p "./shared/${artifact}"

# Process for all machines
for machine in $(jq -r 'keys[]' "$machines"); do
    config=$(jq -r --arg m "$machine" '.[$m]' "$machines")
    # Encrypt for this machine's key
done

# Process for all users
for user in $(jq -r 'keys[]' "$users"); do
    config=$(jq -r --arg u "$user" '.[$u]' "$users")
    # Encrypt for this user's key
done
```

### module.nix

```nix
{ lib, ... }:
with lib;
{
  options.artifacts.config.my-backend = {

    secretsDir = mkOption {
      type = types.path;
      description = "Directory for storing encrypted secrets";
    };

    encryptionKey = mkOption {
      type = types.str;
      description = "Key used for encryption";
    };

    decryptionKey = mkOption {
      type = types.str;
      description = "Key used for decryption";
    };

  };
}
```

---

## Using the Backend in a Flake

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    nixos-artifacts.url = "github:mrvandalo/nixos-artifacts";
    my-backend.url = "path:./my-backend";
  };

  outputs = { nixpkgs, nixos-artifacts, my-backend, ... }: {
    nixosConfigurations.my-machine = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        nixos-artifacts.nixosModules.default
        my-backend.nixosModules.default
        {
          # Set default backend
          artifacts.default.backend.serialization = "my-backend";

          # Configure the backend
          artifacts.config.my-backend = {
            secretsDir = ./secrets;
            encryptionKey = "...";
            decryptionKey = "...";
          };

          # Define artifacts
          artifacts.store.my-secret = {
            files.password = {
              path = "/run/secrets/password";
              owner = "app";
              group = "app";
            };
            generator = pkgs.writers.writeBash "gen" ''
              echo "secret-value" > $out/password
            '';
          };
        }
      ];
    };
  };
}
```

---

## Testing Your Backend

### Minimal Test Backend

For testing without actual encryption:

```toml
[test]
check_serialization = "./check.sh"
serialize = "./serialize.sh"
deserialize = "./deserialize.sh"

[test.capabilities]
serializes = false
```

```bash
# check.sh - always needs generation
#!/usr/bin/env bash
exit 1

# serialize.sh - no-op
#!/usr/bin/env bash
exit 0

# deserialize.sh - no-op
#!/usr/bin/env bash
exit 0
```

### Running Tests

```bash
# From your flake directory
nix run .#artifacts -- tui

# Or filter by machine
nix run .#artifacts -- tui --machine my-machine
```

---

## Error Handling Best Practices

1. **Use `set -euo pipefail`** at the start of all scripts
2. **Validate required environment variables** before using them
3. **Check file existence** before operations
4. **Provide meaningful error messages** on stderr
5. **Clean up temporary files** on both success and failure
6. **Return appropriate exit codes** (0 for success, non-zero for failure)

---

## Security Considerations

1. **Never log secrets** to stdout/stderr
2. **Use secure temporary directories** with proper permissions
3. **Validate input paths** to prevent path traversal
4. **Handle identity files securely** (proper permissions)
5. **Consider key rotation** mechanisms in your design
