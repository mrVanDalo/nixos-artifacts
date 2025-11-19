# CLAUDE.md - AI Assistant Guide for NixOS Artifacts Store

## Project Context

You are working on **NixOS Artifacts Store**, a system that unifies handling of
artifacts (secrets and generated files) in NixOS flakes through a common
abstraction over multiple backends (agenix, sops-nix, colmena, etc.).

## Core Architecture

### Key Components

1. **Artifacts**: Named bundles of files produced by generators, using optional
   user prompts, then serialized into a storage backend
2. **Store**: High-level declarations in NixOS options
   (`artifacts.store.<name>`) containing:
   - Files specification
   - User prompts
   - Generator script/binary
   - Serialization backend reference
3. **Backend**: Technical implementation providing:
   - `serialize` operations
   - `deserialize` operations
   - `check_serialization` program (determines if regeneration is needed)
4. **Machines vs Shared**: Artifacts can be per-machine or shared, with
   directory layout mirroring this split

### Project Structure

```
.
├── README.md              # Design overview, workflows, concepts
├── flake.nix              # Flake outputs, packages, modules, examples
├── modules/               # NixOS modules
│   ├── default.nix        # Module aggregation
│   ├── backend.nix        # Backend serialization options
│   └── store.nix          # Artifacts store tree options
├── pkgs/artifacts-cli/    # Rust CLI implementation (has own CLAUDE.md)
└── examples/              # Example scenarios (backend.toml, flake.nix)
```

## Flake Outputs Reference

- `packages.artifacts-cli-bin` — Pure Rust CLI binary
- `packages.artifacts-cli` — Bash wrapper that:
  - Generates backends.toml from Nix attrset
  - Computes path to make file generator derivation
  - Invokes CLI with proper arguments
- `packages.default` — Points to artifacts-cli
- `nixosModules.default` — Main module system
- `nixosModules.examples` — Example configurations
- `nixosConfigurations.{machine-one,machine-two}` — Test NixOS systems

## Testing & Validation

```bash
nix flake check  # Validate NixOS configuration
nix fmt          # Format code (REQUIRED before commits)
```

## Development Guidelines

### When Making Changes

1. **Keep changes small and focused** — One logical change per commit
2. **Update documentation** — Modify guidelines.md if adding:
   - User-facing commands
   - New options
   - Structural changes
3. **Maintain consistency** — Align terminology with README.md
4. **Format before committing** — Always run `nix fmt`

### Code Organization Principles

- Modules define options structure (`modules/`)
- CLI implements generation/serialization logic (`pkgs/artifacts-cli/`)
- Examples provide test cases and reference implementations (`examples/`)
- Backends are pluggable (test backend wired in flake for development)

## Glossary

- **Artifact**: Logical secret bundle producing one or more deployable files
- **Generator**: Script/binary consuming prompts to produce files in `$out`
- **Backend**: Storage engine with serialize/deserialize/check operations
- **Make**: JSON structure extracted from Nix options that drives the CLI

## Common Tasks

### Understanding the Flow

1. User declares artifacts in NixOS options (`artifacts.store.<name>`)
2. Options are converted to Make JSON structure
3. CLI reads Make JSON and backends.toml
4. Generator produces files based on prompts
5. Backend serializes files to storage
6. Check operations determine when regeneration is needed

### Working with Backends

- Current test backend: Wired in flake.nix for development
- Production targets: agenix, sops-nix, colmena
- Backend must implement: serialize, deserialize, check_serialization

### Modifying Options

- Store options: `modules/store.nix`
- Backend options: `modules/backend.nix`
- Always update guidelines.md with option changes

## Quick Reference

- Primary language: Nix (modules/flake), Rust (CLI)
- Entry point: `flake.nix`
- CLI source: `pkgs/artifacts-cli/` (check its guidelines.md)
- Module system: `modules/default.nix`
- Test configs: `nixosConfigurations.machine-one` and `machine-two`
