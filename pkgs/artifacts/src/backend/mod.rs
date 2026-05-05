//! Backend module for executing scripts and managing artifact serialization.
//!
//! This module provides the core functionality for running external scripts that
//! generate and serialize artifacts.
//!
//! # Architecture
//!
//! Backend scripts are external programs called by the CLI to perform operations:
//!
//! - **Generator scripts**: Produce files in a temporary output directory based on user prompts
//! - **Check scripts**: Determine if an artifact needs regeneration (exit 0 = up-to-date)
//! - **Serialize scripts**: Store generated files in the backend storage system
//!
//! Deserialization (extracting files from backend storage at system activation
//! time) is the responsibility of the backend's NixOS / Home Manager modules,
//! not this CLI.
//!
//! # Security
//!
//! Only the generator runs inside a bubblewrap container — that is the
//! untrusted, user-authored code path. The bwrap container provides:
//! - Separate user namespace with uid/gid 1000
//! - Minimal filesystem access (read-only /nix/store, /bin, /usr/bin)
//! - Temporary /etc/passwd with restricted shell
//! - No network access from within the container
//!
//! Check and serialize scripts run directly on the host: they are part of the
//! backend package the user has already chosen to trust, and they typically
//! need filesystem and network access (e.g. writing into a git repo, calling
//! out to a cloud secret manager) that bwrap would block.
//!
//! # Submodules
//!
//! - [`generator`]: File generation verification and generator script execution
//! - [`serialization`]: Backend script execution for check / serialize operations
//! - [`helpers`]: Utility functions for path resolution, escaping, and validation
//! - [`output_capture`]: Process execution with output capture and timeout support
//! - [`tempfile`]: Temporary file and directory management

pub mod generator;
pub mod helpers;
pub mod output_capture;
pub mod serialization;
pub mod tempfile;
