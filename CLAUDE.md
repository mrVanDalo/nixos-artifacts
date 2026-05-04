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
   - Backend reference
3. **Backend**: Technical implementation providing:
   - `check` script (determines if regeneration is needed)
   - `serialize` script (writes generated files into backend storage)
   - Optional `shared_check` / `shared_serialize` variants for shared artifacts
4. **Machines vs Shared**: Artifacts can be per-machine or shared, with
   directory layout mirroring this split

### Project Structure

```
.
├── README.md                       # Design overview, workflows, concepts
├── flake.nix                       # Flake outputs, packages, modules, examples
├── backends/                       # Reference backend(s) wired into the dev flake
│   ├── default.nix                 # Defines `lib.mkBackend` and `lib.mkArtifactCli`
│   └── test/                       # Test backend used by dev nixosConfigurations
├── modules/                        # NixOS modules
│   ├── default.nix                 # Module aggregation
│   ├── backend.nix                 # `artifacts.default.backend` option
│   ├── store.nix                   # NixOS `artifacts.store` tree (with owner/group)
│   ├── common-store-options.nix    # Shared option fragments for store + hm
│   └── hm/                         # Home Manager module variant
├── pkgs/artifacts/                 # Rust CLI implementation (has own CLAUDE.md)
├── examples/                       # NixOS module fragments demoing artifacts.store
│   ├── simple-generator.nix
│   ├── simple-prompt.nix
│   ├── shared-generator.nix
│   ├── hm/                         # Home Manager equivalents
│   └── ideas/                      # Sketches of scenarios still under design
└── docs/                           # Antora documentation site (has own CLAUDE.md)
```

NOTE: Test scenarios with their own `flake.nix` + `backend.toml` live under
`pkgs/artifacts/examples/scenarios/`, not the top-level `examples/`. See
`pkgs/artifacts/CLAUDE.md` for that catalogue.

## Flake Outputs Reference

- `packages.artifacts-bin` — Pure Rust CLI binary built from `pkgs/artifacts/`
- `packages.artifacts` — Bash wrapper produced by `lib.mkArtifactCli` that sets
  `NIXOS_ARTIFACTS_BACKEND_CONFIG` to a merged `backends.toml` (using
  `include = [...]` directives over each backend package) and exec's
  `artifacts-bin`. The CLI itself evaluates the flake via `nix build` to derive
  the make.json — there is no separate make-file derivation
- `packages.default` — Alias for `packages.artifacts`
- `packages.example-backend` — Test backend produced by `lib.mkBackend` (defined
  under `backends/test/`); used as the default in dev
- `nixosModules.default` — Main module system (imports `modules/`)
- `nixosModules.examples` — Example artifacts.store configurations
- `homeModules.default` — Home Manager module (imports `modules/hm/`)
- `homeModules.examples` — Example home-manager artifacts
- `nixosConfigurations.{machine-one,machine-two}` — Test NixOS systems
- `homeConfigurations.test` — Test home-manager configuration
- `lib.mkBackend` / `lib.mkArtifactCli` — Public Nix API for downstream flakes
  (defined in `backends/default.nix`)

## Testing & Validation

```bash
nix flake check  # Validate NixOS configuration
nix fmt          # Format code (REQUIRED before commits)
```

## Development Guidelines

### When Making Changes

1. **Keep changes small and focused** — One logical change per commit
2. **Update documentation** — Update Antora pages under
   `docs/modules/ROOT/pages/` and the relevant `CLAUDE.md` when adding:
   - User-facing commands
   - New options
   - Structural changes
3. **Maintain consistency** — Align terminology with README.md
4. **Format before committing** — Always run `nix fmt`
5. **Commit without signing** — Always use `git -c commit.gpgsign=false commit`

### Git Commit Messages

Follow this format for all commits and squashed merges:

```
<gitmoji> <short description>

- <detailed point explaining the change>
- <detailed point explaining the change>
```

**Guidelines:**

- Use appropriate gitmoji to categorize the change
- Keep the subject line short and descriptive
- Use bullet points for detailed changes
- Focus on readability and clarity
- Add brief inline comments to code when necessary

### Code Organization Principles

- Modules define options structure (`modules/`)
- CLI implements generation/serialization logic (`pkgs/artifacts/`)
- Examples provide test cases and reference implementations (`examples/`)
- Backends are pluggable (test backend wired in flake for development)

## Glossary

- **Artifact**: Logical secret bundle producing one or more deployable files
- **Generator**: Script/binary consuming prompts to produce files in `$out`
- **Backend**: Storage engine providing `check` and `serialize` scripts
  (per-target: `nixos`, `home`, optional `shared`)
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
- Backend must implement: `check` and `serialize` scripts (and optional
  `shared_check` / `shared_serialize`)

### Modifying Options

- Store options (NixOS): `modules/store.nix` — adds `path`, `owner`, `group`,
  `shared` on top of the common artifact options
- Common store options: `modules/common-store-options.nix` — option fragments
  shared by NixOS and Home Manager (`prompts`, `generator`, `backend`,
  `description`, `name`, file-level `mode`)
- Home Manager store options: `modules/hm/` — reuses `common-store-options.nix`
  minus the system-only fields
- Backend options: `modules/backend.nix` — `artifacts.default.backend`
- Always update the relevant Antora pages under `docs/modules/ROOT/pages/` with
  option changes

## Quick Reference

- Primary language: Nix (modules/flake), Rust (CLI)
- Entry point: `flake.nix`
- CLI source: `pkgs/artifacts/` (check its CLAUDE.md)
- Module system: `modules/default.nix`
- Test configs: `nixosConfigurations.machine-one` and `machine-two`

<!-- BEGIN BEADS INTEGRATION v:1 profile:minimal hash:ca08a54f -->

## Beads Issue Tracker

This project uses **bd (beads)** for issue tracking. Run `bd prime` to see full
workflow context and commands.

### Quick Reference

```bash
bd ready              # Find available work
bd show <id>          # View issue details
bd update <id> --claim  # Claim work
bd close <id>         # Complete work
```

### Rules

- Use `bd` for ALL task tracking — do NOT use TodoWrite, TaskCreate, or markdown
  TODO lists
- Run `bd prime` for detailed command reference and session close protocol
- Use `bd remember` for persistent knowledge — do NOT use MEMORY.md files

## Session Completion

**When ending a work session**, you MUST complete ALL steps below. Work is NOT
complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs
   follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:
   ```bash
   git pull --rebase
   bd dolt push
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**

- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds

<!-- END BEADS INTEGRATION -->
