//! Main test file for the artifacts crate.
//!
//! This file aggregates all test modules for the project.
//!
//! ## Running Tests
//!
//! ```bash
//! # Run all tests
//! cargo test --test tests -- --test-threads=1
//!
//! # Run specific test category
//! cargo test --test tests e2e
//! cargo test --test tests backend
//! cargo test --test tests cli
//!
//! # Run a specific test
//! cargo test --test tests e2e_single_artifact_is_created
//! ```
//!
//! ## Test Requirements (CI)
//!
//! All e2e tests require:
//! - Nix installation with flake support
//! - Scenarios in examples/scenarios/ directory
//! - serial_test for test isolation (#[serial])
//! - Tests run single-threaded (--test-threads=1)
//!
//! ## Test Structure
//!
//! - `async_tests/`: Async runtime and event testing
//! - `backend/`: Backend operation tests (generator, serialization)
//! - `cli/`: CLI command tests (insta-cmd snapshots)
//! - `e2e/`: End-to-end integration tests
//!   - `mod.rs`: Core e2e tests and helpers (TEST-01 to TEST-04)
//!   - `backend_verify.rs`: Backend storage verification (TEST-03, TEST-04)
//!   - `shared_artifact.rs`: Shared artifact tests (TEST-05)
//! - `tui/`: TUI view and interaction tests

mod async_tests;
mod backend;
mod cli;
#[macro_use]
mod common;
mod config;
mod e2e;
mod tui;
