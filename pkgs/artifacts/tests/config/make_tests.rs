//! Snapshot tests for make configuration (make.json) parsing.
//!
//! Each test captures input JSON and parsed output as a readable snapshot.

use std::path::PathBuf;

use artifacts::config::make::MakeConfiguration;

macro_rules! make_snapshot {
    ($input:expr, $config:expr) => {
        format!("Input:\n{}\n\nParsed:\n{:#?}", $input.trim(), $config)
    };
}

#[test]
fn snapshot_empty_configuration() {
    let input = r#"{"nixos": [], "home": []}"#;
    let config = MakeConfiguration::parse_make_config(input, &PathBuf::from("make.json")).unwrap();

    insta::assert_snapshot!(make_snapshot!(input, config));
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
    let config = MakeConfiguration::parse_make_config(input, &PathBuf::from("make.json")).unwrap();

    insta::assert_snapshot!(make_snapshot!(input, config));
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
    let config = MakeConfiguration::parse_make_config(input, &PathBuf::from("make.json")).unwrap();

    insta::assert_snapshot!(make_snapshot!(input, config));
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
    let config = MakeConfiguration::parse_make_config(input, &PathBuf::from("make.json")).unwrap();

    insta::assert_snapshot!(make_snapshot!(input, config));
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
    let config = MakeConfiguration::parse_make_config(input, &PathBuf::from("make.json")).unwrap();

    insta::assert_snapshot!(make_snapshot!(input, config));
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
    let config = MakeConfiguration::parse_make_config(input, &PathBuf::from("make.json")).unwrap();

    insta::assert_snapshot!(make_snapshot!(input, config));
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
    let config = MakeConfiguration::parse_make_config(input, &PathBuf::from("make.json")).unwrap();

    insta::assert_snapshot!(make_snapshot!(input, config));
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
    let config = MakeConfiguration::parse_make_config(input, &PathBuf::from("make.json")).unwrap();

    insta::assert_snapshot!(make_snapshot!(input, config));
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
    let config = MakeConfiguration::parse_make_config(input, &PathBuf::from("make.json")).unwrap();

    insta::assert_snapshot!(make_snapshot!(input, config));
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
    let config = MakeConfiguration::parse_make_config(input, &PathBuf::from("make.json")).unwrap();

    insta::assert_snapshot!(make_snapshot!(input, config));
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
    let config = MakeConfiguration::parse_make_config(input, &PathBuf::from("make.json")).unwrap();

    insta::assert_snapshot!(make_snapshot!(input, config));
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
    let config = MakeConfiguration::parse_make_config(input, &PathBuf::from("make.json")).unwrap();

    insta::assert_snapshot!(make_snapshot!(input, config));
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
    let config = MakeConfiguration::parse_make_config(input, &PathBuf::from("make.json")).unwrap();

    insta::assert_snapshot!(make_snapshot!(input, config));
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
    let config = MakeConfiguration::parse_make_config(input, &PathBuf::from("make.json")).unwrap();

    insta::assert_snapshot!(make_snapshot!(input, config));
}
