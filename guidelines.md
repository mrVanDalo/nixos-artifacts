NixOS Artifacts Store

Project Overview

- Goal: Unify handling of artifacts (secrets and related generated files) in
  NixOS flakes via a common abstraction over multiple backends.
- Backends: Intended to support agenix, sops-nix, colmena (and others). The repo
  provides a test backend wiring in the flake for development.

Key Concepts

- Artifacts: Named bundles of files produced by a generator, possibly using user
  prompts, then serialized into a storage backend.
- Store: The high-level declaration of artifacts in NixOS options
  (artifacts.store.<name>), including files, prompts, generator, and
  serialization backend reference.
- Backend: The technical implementation of serialize/deserialize operations,
  plus a check_serialization program to decide whether generation is needed.
- Machines vs Shared: Artifacts can be per-machine or shared. Directory layout
  mirrors this split in both input and output locations.

Top-Level Layout

- README.md — Design overview, mermaid workflow, concept docs.
- flake.nix — Flake outputs, package wrappers, module wiring, and example NixOS
  configurations.
- modules/ — NixOS modules (options) for artifacts store and backend.
  - default.nix — Imports all module pieces.
  - backend.nix — Declares artifacts.default.backend.serialization.
  - store.nix — Declares artifacts.store tree: files, prompts, generator,
    serialization.
- pkgs/artifacts-cli — Rust crate that builds (has it's own guidelines.md)
- examples/ — Reference scenarios for tests (backend.toml, make.json, scripts).

Nix Flake Outputs (flake.nix)

- packages.artifacts-cli-bin — Builds Rust CLI from pkgs/artifacts-cli.
- packages.artifacts-cli — Wrapper Bash script that:
  - Generates a backends.toml from a Nix attrset (currently wired with a test
    backend).
  - Computes a path to the make file generator derivation.
  - Invokes artifacts-cli "$@" backends.toml MAKE.
- packages.default = artifacts-cli.
- nixosModules.default — Imports ./modules.
- nixosModules.examples — Imports ./examples (example NixOS module wiring).
- nixosConfigurations.{machine-one,machine-two} — Example NixOS systems that set
  artifacts.default.backend.serialization = "test".

Testing

- `nix flake check` to test if nixos project works.

Glossary

- Artifact: A logical secret bundle producing one or more files to deploy.
- Generator: Script/binary that uses prompts to produce files into $out.
- Backend: Storage engine scripts for serialize/deserialize/check.
- Make: The JSON structure extracted from Nix options that drives the CLI.

Contributing Notes for AI

- Keep messages and commit diffs small and well-scoped.
- Update this guidelines.md if you add user-facing commands, options, or
  structure.
- When unsure, sync with README.md terminology and diagrams.
- run `nix fmt` needs to be run after work is done and before commiting.
