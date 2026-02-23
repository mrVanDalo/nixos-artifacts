//! Configuration parsing for the artifacts CLI.
//!
//! This module handles the parsing and loading of configuration from two primary sources:
//!
//! 1. **backend.toml** - Defines serialization backends with their capabilities and scripts.
//!    Backends specify how artifacts are stored, retrieved, and checked for serialization status.
//!    The TOML file supports an `include` directive for splitting configuration across
//!    multiple files.
//!
//! 2. **flake.nix** - Contains NixOS and home-manager configurations with artifact definitions.
//!    The [`make::MakeConfiguration`] structure is extracted from Nix evaluation and contains
//!    all artifact declarations including their files, prompts, generators, and backend
//!    assignments.
//!
//! ## Configuration Flow
//!
//! 1. `backend.toml` is parsed into [`BackendConfiguration`](backend::BackendConfiguration)
//! 2. `flake.nix` is evaluated via Nix to produce JSON
//! 3. JSON is parsed into [`MakeConfiguration`](make::MakeConfiguration)
//! 4. The TUI uses these configurations to drive artifact generation
//!
//! ## Module Structure
//!
//! - [`backend`] - Backend configuration parsing from TOML
//! - [`make`] - Artifact definitions from Nix flake evaluation
//! - [`nix`] - Nix expression building and evaluation helpers

pub mod backend;
pub mod make;
pub mod nix;
