//! Common snapshot macros for insta test assertions.
//!
//! These macros reduce boilerplate for common snapshot filtering patterns.

/// Assert snapshot with temp directory paths redacted.
///
/// Replaces `/tmp/[random]/` paths with `/tmp/FILTERED/` for consistent snapshots.
#[macro_export]
macro_rules! assert_snapshot_temp_filtered {
    ($snapshot:expr) => {
        insta::with_settings!({filters => [
            (r"/tmp/[a-zA-Z0-9._]+/", "/tmp/FILTERED/"),
        ]}, {
            insta::assert_snapshot!($snapshot);
        });
    };
}

/// Assert snapshot with temp directory paths and backend.toml filename redacted.
///
/// Same as `assert_snapshot_temp_filtered!` but also handles the `backend.toml` file path
/// specifically for error messages that include the file name.
#[macro_export]
macro_rules! assert_snapshot_temp_filtered_with_file {
    ($snapshot:expr) => {
        insta::with_settings!({filters => [
            (r"/tmp/[a-zA-Z0-9._]+/backend\.toml", "/tmp/FILTERED/backend.toml"),
        ]}, {
            insta::assert_snapshot!($snapshot);
        });
    };
}

/// Assert snapshot with nix store paths redacted.
///
/// Replaces `/nix/store/[hash]-` paths with `/nix/store/HASH-` for consistent snapshots.
#[macro_export]
macro_rules! assert_snapshot_nix_filtered {
    ($snapshot:expr) => {
        insta::with_settings!({filters => [
            (r"/nix/store/[a-z0-9]+-", "/nix/store/HASH-"),
        ]}, {
            insta::assert_snapshot!($snapshot);
        });
    };
}

/// Assert snapshot with both nix store and temp paths redacted.
///
/// Combines both filter patterns for tests that may have both types of paths.
#[macro_export]
macro_rules! assert_snapshot_nix_and_temp_filtered {
    ($snapshot:expr) => {
        insta::with_settings!({filters => [
            (r"/nix/store/[a-z0-9]+-", "/nix/store/HASH-"),
            (r"/tmp/[a-zA-Z0-9._]+/", "/tmp/FILTERED/"),
        ]}, {
            insta::assert_snapshot!($snapshot);
        });
    };
}

/// Assert debug snapshot with nix store paths redacted.
///
/// Uses `assert_debug_snapshot!` with nix store path filtering.
#[macro_export]
macro_rules! assert_debug_snapshot_nix_filtered {
    ($value:expr) => {
        insta::with_settings!({filters => [
            (r"/nix/store/[a-z0-9]+-", "/nix/store/HASH-"),
        ]}, {
            insta::assert_debug_snapshot!($value);
        });
    };
}

/// Assert debug snapshot with both nix store and temp paths redacted.
///
/// Uses `assert_debug_snapshot!` with both filter patterns.
#[macro_export]
macro_rules! assert_debug_snapshot_nix_and_temp_filtered {
    ($value:expr) => {
        insta::with_settings!({filters => [
            (r"/nix/store/[a-z0-9]+-", "/nix/store/HASH-"),
            (r"/tmp/[a-zA-Z0-9._]+/", "/tmp/FILTERED/"),
        ]}, {
            insta::assert_debug_snapshot!($value);
        });
    };
}
