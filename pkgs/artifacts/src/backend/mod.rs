//! Backend module for executing scripts and managing artifact serialization.
//!
//! This module provides the core functionality for running external scripts that
//! generate, serialize, and deserialize artifacts. All script execution is performed
//! in isolated containers using bubblewrap for security.
//!
//! # Architecture
//!
//! Backend scripts are external programs called by the CLI to perform operations:
//!
//! - **Generator scripts**: Produce files in a temporary output directory based on user prompts
//! - **Check scripts**: Determine if an artifact needs regeneration (exit 0 = up-to-date)
//! - **Serialize scripts**: Store generated files in the backend storage system
//! - **Deserialize scripts**: Extract files from backend storage
//!
//! # Security
//!
//! All script execution uses bubblewrap containers for isolation:
//! - Separate user namespace with uid/gid 1000
//! - Minimal filesystem access (read-only /nix/store, /bin, /usr/bin)
//! - Temporary /etc/passwd with restricted shell
//! - No network access from within containers
//!
//! # Submodules
//!
//! - [`generator`]: File generation verification and generator script execution
//! - [`serialization`]: Backend script execution for check/serialize/deserialize operations
//! - [`helpers`]: Utility functions for path resolution, escaping, and validation
//! - [`output_capture`]: Process execution with output capture and timeout support
//! - [`prompt`]: User prompt handling with interactive and non-interactive modes
//! - [`tempfile`]: Temporary file and directory management

pub mod generator;
pub mod helpers;
pub mod output_capture;
pub mod prompt;
pub mod serialization;
pub mod tempfile;
