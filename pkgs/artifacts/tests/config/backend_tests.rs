//! Snapshot tests for backend configuration parsing.
//!
//! Each test captures input TOML and parsed output as a readable snapshot.

use std::collections::HashSet;
use std::io::Write;
use std::path::PathBuf;
use tempfile::TempDir;

use artifacts::config::backend::{BackendConfiguration, BackendEntry, TargetConfig};

fn create_temp_toml(content: &str) -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let toml_path = temp_dir.path().join("backend.toml");
    let mut file = std::fs::File::create(&toml_path).unwrap();
    file.write_all(content.as_bytes()).unwrap();
    (temp_dir, toml_path)
}

fn load_toml(content: &str) -> BackendConfiguration {
    let (_temp_dir, toml_path) = create_temp_toml(content);
    let mut visited = HashSet::new();
    let config = BackendConfiguration::load_with_includes(&toml_path, &mut visited).unwrap();
    BackendConfiguration {
        config,
        base_path: toml_path.parent().unwrap().to_path_buf(),
        backend_toml: toml_path,
    }
}

fn get_backend<'a>(config: &'a BackendConfiguration, name: &str) -> Option<&'a BackendEntry> {
    config.config.get(name)
}

macro_rules! make_snapshot {
    ($input:expr, $parsed:expr) => {
        format!("Input:\n{}\n\nParsed:\n{:#?}", $input.trim(), $parsed)
    };
}

#[test]
fn snapshot_full_serializing_backend() {
    let input = r#"
[test.nixos]
check = "./check.sh"
serialize = "./serialize.sh"

[test.home]
check = "./check.sh"
serialize = "./serialize.sh"
"#;
    let config = load_toml(input);
    let backend = get_backend(&config, "test").unwrap();

    assert_snapshot_temp_filtered!(make_snapshot!(input, backend));
}

#[test]
fn snapshot_backend_with_shared() {
    let input = r#"
[test.nixos]
check = "./check.sh"
serialize = "./serialize.sh"

[test.home]
check = "./check.sh"
serialize = "./serialize.sh"

[test.shared]
check = "./shared_check.sh"
serialize = "./shared_serialize.sh"
"#;
    let config = load_toml(input);
    let backend = get_backend(&config, "test").unwrap();

    assert_snapshot_temp_filtered!(make_snapshot!(input, backend));
}

#[test]
fn snapshot_passthrough_mode_no_scripts() {
    let input = r#"
[test.nixos]
enabled = true

[test.home]
enabled = true
"#;
    let config = load_toml(input);
    let backend = get_backend(&config, "test").unwrap();

    assert_snapshot_temp_filtered!(make_snapshot!(input, backend));
}

#[test]
fn snapshot_explicit_enabled_false_with_scripts() {
    let input = r#"
[test.nixos]
enabled = false
check = "./check.sh"
serialize = "./serialize.sh"
"#;
    let config = load_toml(input);
    let backend = get_backend(&config, "test").unwrap();

    assert_snapshot_temp_filtered!(make_snapshot!(input, backend));
}

#[test]
fn snapshot_backend_with_settings() {
    let input = r#"
[test.nixos]
check = "./check.sh"
serialize = "./serialize.sh"

[test.settings]
key = "value"
another = 123
"#;
    let config = load_toml(input);
    let backend = get_backend(&config, "test").unwrap();

    assert_snapshot_temp_filtered!(make_snapshot!(input, backend));
}

#[test]
fn snapshot_multiple_backends() {
    let input = r#"
[backend1.nixos]
check = "./check1.sh"
serialize = "./serialize1.sh"

[backend2.nixos]
check = "./check2.sh"
serialize = "./serialize2.sh"

[backend2.home]
check = "./check2.sh"
serialize = "./serialize2.sh"
"#;
    let config = load_toml(input);

    let mut keys: Vec<_> = config.config.keys().collect();
    keys.sort();

    let backend1 = get_backend(&config, "backend1").unwrap();
    let backend2 = get_backend(&config, "backend2").unwrap();
    let snapshot = format!(
        "Input:\n{}\n\nBackends: {:?}\n\nbackend1:\n{:#?}\n\nbackend2:\n{:#?}",
        input.trim(),
        keys,
        backend1,
        backend2
    );
    assert_snapshot_temp_filtered!(snapshot);
}

#[test]
fn snapshot_empty_backend_section() {
    let input = r#"
[test]
"#;
    let config = load_toml(input);
    let backend = get_backend(&config, "test").unwrap();

    assert_snapshot_temp_filtered!(make_snapshot!(input, backend));
}

#[test]
fn snapshot_partial_backend_nixos_only() {
    let input = r#"
[test.nixos]
check = "./check.sh"
serialize = "./serialize.sh"
"#;
    let config = load_toml(input);
    let backend = get_backend(&config, "test").unwrap();

    assert_snapshot_temp_filtered!(make_snapshot!(input, backend));
}

#[test]
fn snapshot_partial_backend_home_only() {
    let input = r#"
[test.home]
check = "./check.sh"
serialize = "./serialize.sh"
"#;
    let config = load_toml(input);
    let backend = get_backend(&config, "test").unwrap();

    assert_snapshot_temp_filtered!(make_snapshot!(input, backend));
}

#[test]
fn snapshot_disabled_shared_target() {
    let input = r#"
[test.nixos]
check = "./check.sh"
serialize = "./serialize.sh"

[test.shared]
enabled = false
check = "./shared_check.sh"
serialize = "./shared_serialize.sh"
"#;
    let config = load_toml(input);
    let backend = get_backend(&config, "test").unwrap();

    assert_snapshot_temp_filtered!(make_snapshot!(input, backend));
}

#[test]
fn snapshot_inferred_enabled_from_scripts() {
    let input = r#"
[test.nixos]
check = "./check.sh"
serialize = "./serialize.sh"

[test.home]
enabled = true
"#;
    let config = load_toml(input);
    let backend = get_backend(&config, "test").unwrap();

    assert_snapshot_temp_filtered!(make_snapshot!(input, backend));
}

#[test]
fn snapshot_include_directive() {
    let included = r#"
[included.nixos]
check = "./included_check.sh"
serialize = "./included_serialize.sh"
"#;
    let main = r#"
include = ["./included.toml"]

[main.nixos]
check = "./main_check.sh"
serialize = "./main_serialize.sh"
"#;

    let temp_dir = TempDir::new().unwrap();
    let included_path = temp_dir.path().join("included.toml");
    let mut file = std::fs::File::create(&included_path).unwrap();
    file.write_all(included.as_bytes()).unwrap();

    let main_path = temp_dir.path().join("backend.toml");
    let mut file = std::fs::File::create(&main_path).unwrap();
    file.write_all(main.as_bytes()).unwrap();

    let mut visited = HashSet::new();
    let config = BackendConfiguration::load_with_includes(&main_path, &mut visited).unwrap();

    let mut keys: Vec<_> = config.keys().collect();
    keys.sort();

    let snapshot = format!(
        "Included file:\n{}\n\nMain file:\n{}\n\nBackends: {:?}",
        included.trim(),
        main.trim(),
        keys
    );
    insta::assert_snapshot!(snapshot);
}

fn load_toml_result(content: &str) -> anyhow::Result<BackendConfiguration> {
    let (_temp_dir, toml_path) = create_temp_toml(content);
    let mut visited = HashSet::new();
    let config = BackendConfiguration::load_with_includes(&toml_path, &mut visited)?;
    Ok(BackendConfiguration {
        config,
        base_path: toml_path.parent().unwrap().to_path_buf(),
        backend_toml: toml_path,
    })
}

#[test]
fn snapshot_error_check_without_serialize() {
    let input = r#"
[test.nixos]
check = "./check.sh"
"#;
    let result = load_toml_result(input);

    let snapshot = format!("Input:\n{}\n\nResult:\n{:?}", input.trim(), result);
    assert_snapshot_temp_filtered_with_file!(snapshot);
}

#[test]
fn snapshot_error_serialize_without_check() {
    let input = r#"
[test.nixos]
serialize = "./serialize.sh"
"#;
    let result = load_toml_result(input);

    let snapshot = format!("Input:\n{}\n\nResult:\n{:?}", input.trim(), result);
    assert_snapshot_temp_filtered_with_file!(snapshot);
}

#[test]
fn snapshot_validate_shared_serialize_missing() {
    let input = r#"
[test.nixos]
check = "./check.sh"
serialize = "./serialize.sh"
"#;
    let config = load_toml(input);
    let result = config.validate_shared_serialize("test");

    let snapshot = format!("Input:\n{}\n\nResult:\n{:?}", input.trim(), result);
    assert_snapshot_temp_filtered_with_file!(snapshot);
}

#[test]
fn snapshot_validate_shared_serialize_present() {
    let input = r#"
[test.nixos]
check = "./check.sh"
serialize = "./serialize.sh"

[test.shared]
check = "./shared_check.sh"
serialize = "./shared_serialize.sh"
"#;
    let config = load_toml(input);
    let result = config.validate_shared_serialize("test");

    let snapshot = format!("Input:\n{}\n\nResult:\n{:?}", input.trim(), result);
    assert_snapshot_temp_filtered_with_file!(snapshot);
}

#[test]
fn snapshot_target_config_serializes_true() {
    let config = TargetConfig {
        enabled: None,
        check: Some("./check.sh".to_string()),
        serialize: Some("./serialize.sh".to_string()),
    };

    let snapshot = format!(
        "TargetConfig:\n{:#?}\n\nserializes: {}\nis_enabled: {}",
        config,
        config.serializes(),
        config.is_enabled()
    );
    insta::assert_snapshot!(snapshot);
}

#[test]
fn snapshot_target_config_passthrough() {
    let config = TargetConfig {
        enabled: Some(true),
        check: None,
        serialize: None,
    };

    let snapshot = format!(
        "TargetConfig:\n{:#?}\n\nserializes: {}\nis_enabled: {}",
        config,
        config.serializes(),
        config.is_enabled()
    );
    insta::assert_snapshot!(snapshot);
}

#[test]
fn snapshot_target_config_disabled_explicit() {
    let config = TargetConfig {
        enabled: Some(false),
        check: Some("./check.sh".to_string()),
        serialize: Some("./serialize.sh".to_string()),
    };

    let snapshot = format!(
        "TargetConfig:\n{:#?}\n\nserializes: {}\nis_enabled: {}",
        config,
        config.serializes(),
        config.is_enabled()
    );
    insta::assert_snapshot!(snapshot);
}

#[test]
fn snapshot_target_config_validation_success() {
    let config = TargetConfig {
        enabled: None,
        check: Some("./check.sh".to_string()),
        serialize: Some("./serialize.sh".to_string()),
    };
    let result = config.validate(artifacts::config::backend::TargetType::NixOS, "test");

    let snapshot = format!(
        "TargetConfig:\n{:#?}\n\nValidation result:\n{:?}",
        config, result
    );
    insta::assert_snapshot!(snapshot);
}

#[test]
fn snapshot_target_config_validation_check_without_serialize() {
    let config = TargetConfig {
        enabled: None,
        check: Some("./check.sh".to_string()),
        serialize: None,
    };
    let result = config.validate(artifacts::config::backend::TargetType::NixOS, "test");

    let snapshot = format!(
        "TargetConfig:\n{:#?}\n\nValidation result:\n{:?}",
        config, result
    );
    insta::assert_snapshot!(snapshot);
}

#[test]
fn snapshot_target_config_validation_serialize_without_check() {
    let config = TargetConfig {
        enabled: None,
        check: None,
        serialize: Some("./serialize.sh".to_string()),
    };
    let result = config.validate(artifacts::config::backend::TargetType::Home, "test");

    let snapshot = format!(
        "TargetConfig:\n{:#?}\n\nValidation result:\n{:?}",
        config, result
    );
    insta::assert_snapshot!(snapshot);
}
