//! Snapshot tests for make configuration (make.json) parsing.
//!
//! Each test captures input JSON and parsed output as a readable snapshot.

use std::io::Write;
use std::path::PathBuf;
use tempfile::TempDir;

use artifacts::config::make::MakeConfiguration;

fn create_temp_make_json(content: &str) -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let json_path = temp_dir.path().join("make.json");
    let mut file = std::fs::File::create(&json_path).unwrap();
    file.write_all(content.as_bytes()).unwrap();
    (temp_dir, json_path)
}

macro_rules! make_snapshot {
    ($input:expr, $parsed:expr) => {
        format!("Input:\n{}\n\nParsed:\n{:#?}", $input.trim(), $parsed)
    };
}

#[test]
fn snapshot_empty_configuration() {
    let input = r#"{"nixos": [], "home": []}"#;
    let (_temp_dir, json_path) = create_temp_make_json(input);
    let config = MakeConfiguration::read_make_config(&json_path).unwrap();
    
    let snapshot = format!(
        "Input:\n{}\n\nnixos_map: {:#?}\n\nhome_map: {:#?}",
        input,
        config.nixos_map,
        config.home_map
    );
    insta::assert_snapshot!(snapshot);
}

#[test]
fn snapshot_single_nixos_artifact_no_shared() {
    let input = r#"{
    "nixos": [{
        "machine": "machine-one",
        "artifacts": {
            "my-secret": {
                "name": "my-secret",
                "shared": false,
                "files": {},
                "prompts": {},
                "generator": "/nix/store/gen.sh",
                "serialization": "test"
            }
        },
        "config": {}
    }],
    "home": []
}"#;
    let (_temp_dir, json_path) = create_temp_make_json(input);
    let config = MakeConfiguration::read_make_config(&json_path).unwrap();
    let shared = config.get_shared_artifacts();
    let artifact = config.nixos_map.get("machine-one").unwrap().get("my-secret").unwrap();
    
    let snapshot = make_snapshot!(input, artifact);
    insta::assert_snapshot!(snapshot);
    
    let shared_snapshot = make_snapshot!(input, shared);
    insta::assert_snapshot!("single_nixos_artifact_shared", shared_snapshot);
}

#[test]
fn snapshot_shared_artifact_single_machine() {
    let input = r#"{
    "nixos": [{
        "machine": "machine-one",
        "artifacts": {
            "shared-secret": {
                "name": "shared-secret",
                "shared": true,
                "files": {},
                "prompts": {},
                "generator": "/nix/store/gen.sh",
                "serialization": "test"
            }
        },
        "config": {}
    }],
    "home": []
}"#;
    let (_temp_dir, json_path) = create_temp_make_json(input);
    let config = MakeConfiguration::read_make_config(&json_path).unwrap();
    let shared = config.get_shared_artifacts();
    let info = shared.get("shared-secret").unwrap();
    
    let snapshot = make_snapshot!(input, info);
    insta::assert_snapshot!(snapshot);
}

#[test]
fn snapshot_shared_artifact_multiple_machines_same_generator() {
    let input = r#"{
    "nixos": [
        {
            "machine": "machine-one",
            "artifacts": {
                "shared-secret": {
                    "name": "shared-secret",
                    "shared": true,
                    "files": {},
                    "prompts": {},
                    "generator": "/nix/store/gen.sh",
                    "serialization": "test"
                }
            },
            "config": {}
        },
        {
            "machine": "machine-two",
            "artifacts": {
                "shared-secret": {
                    "name": "shared-secret",
                    "shared": true,
                    "files": {},
                    "prompts": {},
                    "generator": "/nix/store/gen.sh",
                    "serialization": "test"
                }
            },
            "config": {}
        }
    ],
    "home": []
}"#;
    let (_temp_dir, json_path) = create_temp_make_json(input);
    let config = MakeConfiguration::read_make_config(&json_path).unwrap();
    let shared = config.get_shared_artifacts();
    let info = shared.get("shared-secret").unwrap();
    
    let snapshot = make_snapshot!(input, info);
    insta::assert_snapshot!(snapshot);
}

#[test]
fn snapshot_shared_artifact_multiple_machines_different_generators() {
    let input = r#"{
    "nixos": [
        {
            "machine": "machine-one",
            "artifacts": {
                "shared-secret": {
                    "name": "shared-secret",
                    "shared": true,
                    "files": {},
                    "prompts": {},
                    "generator": "/nix/store/gen-a.sh",
                    "serialization": "test"
                }
            },
            "config": {}
        },
        {
            "machine": "machine-two",
            "artifacts": {
                "shared-secret": {
                    "name": "shared-secret",
                    "shared": true,
                    "files": {},
                    "prompts": {},
                    "generator": "/nix/store/gen-b.sh",
                    "serialization": "test"
                }
            },
            "config": {}
        }
    ],
    "home": []
}"#;
    let (_temp_dir, json_path) = create_temp_make_json(input);
    let config = MakeConfiguration::read_make_config(&json_path).unwrap();
    let shared = config.get_shared_artifacts();
    let info = shared.get("shared-secret").unwrap();
    
    let snapshot = make_snapshot!(input, info);
    insta::assert_snapshot!(snapshot);
}

#[test]
fn snapshot_shared_artifact_mixed_nixos_and_home() {
    let input = r#"{
    "nixos": [{
        "machine": "server",
        "artifacts": {
            "shared-secret": {
                "name": "shared-secret",
                "shared": true,
                "files": {},
                "prompts": {},
                "generator": "/nix/store/gen.sh",
                "serialization": "test"
            }
        },
        "config": {}
    }],
    "home": [{
        "user": "alice@workstation",
        "artifacts": {
            "shared-secret": {
                "name": "shared-secret",
                "shared": true,
                "files": {},
                "prompts": {},
                "generator": "/nix/store/gen.sh",
                "serialization": "test"
            }
        },
        "config": {}
    }]
}"#;
    let (_temp_dir, json_path) = create_temp_make_json(input);
    let config = MakeConfiguration::read_make_config(&json_path).unwrap();
    let shared = config.get_shared_artifacts();
    let info = shared.get("shared-secret").unwrap();
    
    let snapshot = make_snapshot!(input, info);
    insta::assert_snapshot!(snapshot);
}

#[test]
fn snapshot_shared_artifact_matching_files_no_error() {
    let input = r#"{
    "nixos": [
        {
            "machine": "machine-one",
            "artifacts": {
                "shared-secret": {
                    "name": "shared-secret",
                    "shared": true,
                    "files": {
                        "id_ed25519": {
                            "name": "id_ed25519",
                            "path": "/run/secrets/id_ed25519",
                            "owner": "root",
                            "group": "root"
                        }
                    },
                    "prompts": {},
                    "generator": "/nix/store/gen.sh",
                    "serialization": "test"
                }
            },
            "config": {}
        },
        {
            "machine": "machine-two",
            "artifacts": {
                "shared-secret": {
                    "name": "shared-secret",
                    "shared": true,
                    "files": {
                        "id_ed25519": {
                            "name": "id_ed25519",
                            "path": "/run/secrets/id_ed25519",
                            "owner": "root",
                            "group": "root"
                        }
                    },
                    "prompts": {},
                    "generator": "/nix/store/gen.sh",
                    "serialization": "test"
                }
            },
            "config": {}
        }
    ],
    "home": []
}"#;
    let (_temp_dir, json_path) = create_temp_make_json(input);
    let config = MakeConfiguration::read_make_config(&json_path).unwrap();
    let shared = config.get_shared_artifacts();
    let info = shared.get("shared-secret").unwrap();
    
    let snapshot = make_snapshot!(input, info);
    insta::assert_snapshot!(snapshot);
}

#[test]
fn snapshot_shared_artifact_mismatched_files_error() {
    let input = r#"{
    "nixos": [
        {
            "machine": "machine-one",
            "artifacts": {
                "shared-secret": {
                    "name": "shared-secret",
                    "shared": true,
                    "files": {
                        "id_ed25519": {
                            "name": "id_ed25519",
                            "path": "/run/secrets/id_ed25519",
                            "owner": "root",
                            "group": "root"
                        }
                    },
                    "prompts": {},
                    "generator": "/nix/store/gen.sh",
                    "serialization": "test"
                }
            },
            "config": {}
        },
        {
            "machine": "machine-two",
            "artifacts": {
                "shared-secret": {
                    "name": "shared-secret",
                    "shared": true,
                    "files": {
                        "id_ed25519": {
                            "name": "id_ed25519",
                            "path": "/run/secrets/id_ed25519",
                            "owner": "root",
                            "group": "root"
                        },
                        "id_ed25519.pub": {
                            "name": "id_ed25519.pub",
                            "path": "/run/secrets/id_ed25519.pub",
                            "owner": "root",
                            "group": "root"
                        }
                    },
                    "prompts": {},
                    "generator": "/nix/store/gen.sh",
                    "serialization": "test"
                }
            },
            "config": {}
        }
    ],
    "home": []
}"#;
    let (_temp_dir, json_path) = create_temp_make_json(input);
    let config = MakeConfiguration::read_make_config(&json_path).unwrap();
    let shared = config.get_shared_artifacts();
    let info = shared.get("shared-secret").unwrap();
    
    let snapshot = make_snapshot!(input, info);
    insta::assert_snapshot!(snapshot);
}

#[test]
fn snapshot_shared_artifact_different_file_names_error() {
    let input = r#"{
    "nixos": [
        {
            "machine": "machine-one",
            "artifacts": {
                "shared-secret": {
                    "name": "shared-secret",
                    "shared": true,
                    "files": {
                        "secret-a": {
                            "name": "secret-a",
                            "path": "/run/secrets/secret-a",
                            "owner": "root",
                            "group": "root"
                        }
                    },
                    "prompts": {},
                    "generator": "/nix/store/gen.sh",
                    "serialization": "test"
                }
            },
            "config": {}
        },
        {
            "machine": "machine-two",
            "artifacts": {
                "shared-secret": {
                    "name": "shared-secret",
                    "shared": true,
                    "files": {
                        "secret-b": {
                            "name": "secret-b",
                            "path": "/run/secrets/secret-b",
                            "owner": "root",
                            "group": "root"
                        }
                    },
                    "prompts": {},
                    "generator": "/nix/store/gen.sh",
                    "serialization": "test"
                }
            },
            "config": {}
        }
    ],
    "home": []
}"#;
    let (_temp_dir, json_path) = create_temp_make_json(input);
    let config = MakeConfiguration::read_make_config(&json_path).unwrap();
    let shared = config.get_shared_artifacts();
    let info = shared.get("shared-secret").unwrap();
    
    let snapshot = make_snapshot!(input, info);
    insta::assert_snapshot!(snapshot);
}

#[test]
fn snapshot_artifact_with_description() {
    let input = r#"{
    "nixos": [{
        "machine": "machine-one",
        "artifacts": {
            "test-secret": {
                "name": "test-secret",
                "description": "Test artifact description",
                "shared": false,
                "files": {},
                "prompts": {},
                "generator": "/nix/store/gen.sh",
                "serialization": "test"
            }
        },
        "config": {}
    }],
    "home": []
}"#;
    let (_temp_dir, json_path) = create_temp_make_json(input);
    let config = MakeConfiguration::read_make_config(&json_path).unwrap();
    let artifact = config.nixos_map.get("machine-one").unwrap().get("test-secret").unwrap();
    
    let snapshot = make_snapshot!(input, artifact);
    insta::assert_snapshot!(snapshot);
}

#[test]
fn snapshot_artifact_without_description() {
    let input = r#"{
    "nixos": [{
        "machine": "machine-one",
        "artifacts": {
            "test-secret": {
                "name": "test-secret",
                "shared": false,
                "files": {},
                "prompts": {},
                "generator": "/nix/store/gen.sh",
                "serialization": "test"
            }
        },
        "config": {}
    }],
    "home": []
}"#;
    let (_temp_dir, json_path) = create_temp_make_json(input);
    let config = MakeConfiguration::read_make_config(&json_path).unwrap();
    let artifact = config.nixos_map.get("machine-one").unwrap().get("test-secret").unwrap();
    
    let snapshot = make_snapshot!(input, artifact);
    insta::assert_snapshot!(snapshot);
}

#[test]
fn snapshot_shared_artifact_with_description() {
    let input = r#"{
    "nixos": [
        {
            "machine": "machine-one",
            "artifacts": {
                "shared-secret": {
                    "name": "shared-secret",
                    "description": "Shared SSH key for all servers",
                    "shared": true,
                    "files": {},
                    "prompts": {},
                    "generator": "/nix/store/gen.sh",
                    "serialization": "test"
                }
            },
            "config": {}
        },
        {
            "machine": "machine-two",
            "artifacts": {
                "shared-secret": {
                    "name": "shared-secret",
                    "description": "Shared SSH key for all servers",
                    "shared": true,
                    "files": {},
                    "prompts": {},
                    "generator": "/nix/store/gen.sh",
                    "serialization": "test"
                }
            },
            "config": {}
        }
    ],
    "home": []
}"#;
    let (_temp_dir, json_path) = create_temp_make_json(input);
    let config = MakeConfiguration::read_make_config(&json_path).unwrap();
    let shared = config.get_shared_artifacts();
    let info = shared.get("shared-secret").unwrap();
    
    let snapshot = make_snapshot!(input, info);
    insta::assert_snapshot!(snapshot);
}

#[test]
fn snapshot_artifact_with_files_and_prompts() {
    let input = r#"{
    "nixos": [{
        "machine": "server",
        "artifacts": {
            "ssh-key": {
                "name": "ssh-key",
                "description": "SSH key for authentication",
                "shared": false,
                "files": {
                    "id_ed25519": {
                        "name": "id_ed25519",
                        "path": "/run/secrets/id_ed25519",
                        "owner": "root",
                        "group": "root"
                    },
                    "id_ed25519.pub": {
                        "name": "id_ed25519.pub",
                        "path": "/run/secrets/id_ed25519.pub",
                        "owner": "root",
                        "group": "root"
                    }
                },
                "prompts": {
                    "passphrase": {
                        "name": "passphrase",
                        "description": "SSH key passphrase"
                    },
                    "comment": {
                        "name": "comment",
                        "description": "Key comment"
                    }
                },
                "generator": "/nix/store/gen.sh",
                "serialization": "agenix"
            }
        },
        "config": {
            "agenix": {
                "publicKey": "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAA..."
            }
        }
    }],
    "home": []
}"#;
    let (_temp_dir, json_path) = create_temp_make_json(input);
    let config = MakeConfiguration::read_make_config(&json_path).unwrap();
    let artifact = config.nixos_map.get("server").unwrap().get("ssh-key").unwrap();
    let backend_config = config.get_backend_config_for("server", "agenix");
    
    let artifact_snapshot = make_snapshot!(input, artifact);
    insta::assert_snapshot!(artifact_snapshot);
    
    let config_snapshot = format!("Input:\n{}\n\nBackend config (agenix):\n{:#?}", input.trim(), backend_config);
    insta::assert_snapshot!("artifact_with_files_prompts_config", config_snapshot);
}

#[test]
fn snapshot_home_manager_configuration() {
    let input = r#"{
    "nixos": [],
    "home": [{
        "user": "alice@workstation",
        "artifacts": {
            "user-secret": {
                "name": "user-secret",
                "shared": false,
                "files": {
                    "config.json": {
                        "name": "config.json",
                        "path": "~/.config/app/config.json"
                    }
                },
                "prompts": {
                    "token": {
                        "name": "token",
                        "description": "API token"
                    }
                },
                "generator": "/nix/store/gen.sh",
                "serialization": "test"
            }
        },
        "config": {}
    }]
}"#;
    let (_temp_dir, json_path) = create_temp_make_json(input);
    let config = MakeConfiguration::read_make_config(&json_path).unwrap();
    let artifact = config.home_map.get("alice@workstation").unwrap().get("user-secret").unwrap();
    
    let artifact_snapshot = make_snapshot!(input, artifact);
    insta::assert_snapshot!(artifact_snapshot);
    
    let nixos_snapshot = format!("Input:\n{}\n\nnixos_map (empty):\n{:#?}", input.trim(), config.nixos_map);
    insta::assert_snapshot!("home_manager_nixos_empty", nixos_snapshot);
}