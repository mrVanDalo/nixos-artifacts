# External Integrations

**Analysis Date:** 2025-02-13

## APIs & External Services

**Documentation Hosting:**

- **GitHub Pages** - Documentation deployment target
  - Workflow: `.github/workflows/publish-pages.yml`
  - Trigger: Push to main branch or manual dispatch
  - Extensions: `@sntke/antora-mermaid-extension`, `@antora/lunr-extension`

**Documentation Tools:**

- **Antora** - Documentation site generator
  - Source: GitLab UI bundle (remote URL)
  - Playbook: `docs/antora-playbook.yml`
  - Extensions: Mermaid diagrams, Lunr search

**Nix Ecosystem:**

- **nixpkgs** (nixos-unstable) - Base package repository
  - Input: `github:NixOS/nixpkgs/nixos-unstable`
- **flake-parts** - Flake composition framework
  - Input: `github:hercules-ci/flake-parts`
- **devshell** - Development environment
  - Input: `github:numtide/devshell`
- **treefmt-nix** - Formatting orchestration
  - Input: `github:numtide/treefmt-nix`
- **home-manager** - Home configuration management
  - Input: `github:nix-community/home-manager`
- **antora-flake** - Documentation flake helper
  - Input: `github:mrvandalo/antora-flake`

## Data Storage

**Databases:**

- Not applicable - No database integration detected

**File Storage:**

- **Local filesystem** - Artifact storage via configurable backends
- **Backend-dependent storage** - Delegated to backend implementations (agenix,
  sops-nix, colmena, etc.)
- Temporary files: Managed via `tempfile` crate in Rust

**Caching:**

- Antora cache: `$PWD/.cache` during documentation builds
- Cargo cache: Standard Rust build cache

## Authentication & Identity

**Auth Provider:**

- Not applicable - CLI tool operates locally
- File permissions managed via NixOS/home-manager (owner/group settings)

## Monitoring & Observability

**Error Tracking:**

- `anyhow` crate for error context and propagation
- `log` crate with std feature for structured logging
- Console output via TUI (ratatui)

**Logs:**

- stdout/stderr capture in `src/backend/output_capture.rs`
- Log levels via `log` crate
- No external logging service integration

## CI/CD & Deployment

**Hosting:**

- GitHub Actions for documentation publishing
- Nix flakes for package distribution

**CI Pipeline:**

- **GitHub Actions workflow** (`.github/workflows/publish-pages.yml`)
  - Node.js 20 setup
  - Antora installation via npm
  - Site generation with extensions
  - GitHub Pages deployment

**Deployment Targets:**

- GitHub Pages (documentation)
- Nix binary cache (packages via flake)

## Environment Configuration

**Required env vars:**

- `NIXOS_ARTIFACTS_BACKEND_CONFIG` - Generated backends.toml path (set by
  wrapper)
- `RUSTUP_HOME` / `CARGO_HOME` - Rust toolchain (docs build)
- `RUSTUP_TOOLCHAIN` - Pinned to 1.87.0

**Runtime environment (backend scripts):**

- `$inputs` - Directory with file metadata JSON
- `$prompts` - Directory with prompt responses
- `$out` - Generator output directory
- `$machine` - Target machine name
- `$artifact` - Artifact identifier
- `$machines` / `$users` - JSON mappings (shared artifacts only)

**Secrets location:**

- Backend-dependent (agenix, sops-nix, etc.)
- Test backend outputs to stdout
- No centralized secrets management detected

## Webhooks & Callbacks

**Incoming:**

- None detected

**Outgoing:**

- None detected

## Backend Integration Points

**Backend Interface:**

- Scripts invoked by CLI with standardized environment
- `check_serialization` - Determine if regeneration needed
- `serialize` - Store generated artifacts
- `deserialize` - Retrieve artifacts (optional)
- Container isolation via bubblewrap for security

**Supported Backend Types (planned/mentioned):**

- agenix - Age-based encryption
- sops-nix - Mozilla SOPS integration
- colmena - Distributed deployment
- test - Development/testing backend

---

_Integration audit: 2025-02-13_
