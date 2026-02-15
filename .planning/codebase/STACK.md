# Technology Stack

**Analysis Date:** 2025-02-13

## Languages

**Primary:**

- **Rust** 1.87.0 - CLI implementation (`pkgs/artifacts/src/`)
  - Edition 2024
  - Single binary target: `src/bin/artifacts.rs`

**Secondary:**

- **Nix** - Module definitions, flake configuration, build system
  - NixOS modules: `modules/`
  - Home-manager modules: `modules/hm/`
- **Bash** - Backend scripts, wrapper scripts, generators
- **AsciiDoc** - Documentation source files (`docs/modules/ROOT/pages/`)

## Runtime

**Environment:**

- Nix Flakes with `flake-parts` framework
- Supported systems: `x86_64-linux`, `aarch64-linux`

**Package Manager:**

- **Cargo** - Rust dependency management
- **Nix** - System-level dependency management
- Lockfile: `pkgs/artifacts/Cargo.lock` present

## Frameworks

**Core:**

- **ratatui** 0.29 - Terminal UI framework for Rust
- **crossterm** 0.28 - Cross-platform terminal manipulation
- **clap** 4 - Command-line argument parsing with derive macros
- **serde** 1 - Serialization/deserialization framework

**Testing:**

- **insta** 1.43.1 - Snapshot testing with filters support
- **insta-cmd** 0.6 - Command-line snapshot testing
- **serial_test** 3 - Sequential test execution
- **tempfile** 3 - Temporary file/directory management

**Build/Dev:**

- **flake-parts** - Modular flake composition
- **devshell** - Development environment management
- **treefmt-nix** - Code formatting orchestration
- **antora** 3.x - Documentation site generator (Node.js-based)

## Key Dependencies

**Critical:**

- `anyhow` 1 - Error handling and propagation
- `serde_json` 1 - JSON serialization
- `toml` 0.8 - TOML configuration parsing
- `which` 6 - Executable path resolution
- `log` 0.4 - Logging framework with std and serde features

**Infrastructure:**

- `bubblewrap` - Container isolation for generator/serialize scripts
- `nixpkgs` (nixos-unstable) - Base package set
- `home-manager` - Home configuration management

## Configuration

**Environment:**

- `NIXOS_ARTIFACTS_BACKEND_CONFIG` - Path to generated backends.toml
- `NIXOS_ARTIFACTS_PROJECT_ROOT` - Project root for development

**Build:**

- `flake.nix` - Main flake definition
- `nix/devshells.nix` - Development shell configuration
- `nix/docs.nix` - Documentation build scripts
- `nix/formatter.nix` - treefmt configuration
- `nix/options.nix` - NixOS options documentation

**Backend Configuration:**

- TOML-based backend definitions (`backend.toml`)
- Supports include directives for splitting configuration
- Scripts for check_serialization, serialize, deserialize operations

## Platform Requirements

**Development:**

- Nix with flakes enabled
- Rust toolchain (1.87.0+ for edition 2024)
- Node.js 20+ (for Antora documentation)
- bubblewrap (for containerized script execution)

**Production:**

- NixOS system or home-manager installation
- Target architectures: x86_64-linux, aarch64-linux
- Requires flake.nix with artifact store definitions

---

_Stack analysis: 2025-02-13_
