{
  description = "Description for the project";

  inputs = {
    devshell.url = "github:numtide/devshell";
    flake-parts.url = "github:hercules-ci/flake-parts";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    treefmt-nix.inputs.nixpkgs.follows = "nixpkgs";
    treefmt-nix.url = "github:numtide/treefmt-nix";
    antora-flake.url = "github:mrvandalo/antora-flake";
    antora-flake.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs =
    inputs@{ flake-parts, self, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [
        ./nix/formatter.nix
        ./nix/devshells.nix
        ./nix/options.nix
      ];
      perSystem =
        {
          pkgs,
          self',
          system,
          lib,
          ...
        }:
        {

          packages.default = self'.packages.artifacts-cli;
          packages.artifacts-cli-bin = pkgs.callPackage ./pkgs/artifacts-cli { };

          packages.artifacts-cli =
            pkgs.callPackage
              (
                {
                  backends ? { },
                }:
                let
                  backendsFile = (pkgs.formats.toml { }).generate "backends.toml" backends;
                in
                pkgs.writers.writeBashBin "artifacts" ''
                  set -e
                  set -o pipefail
                  export NIXOS_ARTIFACTS_BACKEND_CONFIG=${backendsFile}
                  ${self'.packages.artifacts-cli-bin}/bin/artifacts "$@"
                ''
              )
              {
                backends = {
                  test.check_serialization = pkgs.writers.writeBash "test_check" "exit 1"; # always fail
                  test.deserialize = pkgs.writers.writeBash "test_deserialize" ''
                    echo "what is deserialization there for again?";
                  '';
                  test.serialize = pkgs.writers.writeBash "test_serialize" ''
                    for file in "$out"/*; do
                        if [ -f "$file" ]; then
                            echo "=== Content of $file ==="
                            cat "$file"
                            echo "========================="
                        fi
                    done
                  '';
                };
              };

          apps = {
            build-docs = {
              type = "app";
              program = "${
                pkgs.writeShellApplication {
                  name = "build-docs";
                  runtimeInputs = [
                    pkgs.antora
                    pkgs.git
                    pkgs.nodejs
                  ];
                  text = ''
                    set -euo pipefail
                    export ANTORA_CACHE_DIR="$PWD/.cache"
                    antora \
                      --stacktrace \
                      --to-dir /tmp/antora-public \
                      --extension ${pkgs.antora-lunr-extension}/node_modules/@antora/lunr-extension \
                      --extension ${
                        inputs.antora-flake.packages.${system}.antora-mermaid-extension
                      }/lib/node_modules/@sntke/antora-mermaid-extension \
                      antora-playbook.yml
                    echo
                    echo "Site generated in: docs/public"
                  '';
                }
              }/bin/build-docs";
            };
          };
        };
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];

      flake = {

        nixosConfigurations =
          let
            machineConfiguration =
              name:
              inputs.nixpkgs.lib.nixosSystem {
                system = "x86_64-linux";
                specialArgs = { inherit inputs; };
                modules = [
                  self.nixosModules.default
                  self.nixosModules.examples
                  (
                    { pkgs, config, ... }:
                    {
                      networking.hostName = name;
                      artifacts.default.backend.serialization = "test";
                    }
                  )
                ];
              };
          in
          {
            machine-one = machineConfiguration "machine-one";
            machine-two = machineConfiguration "machine-two";
          };

        nixosModules.default = {
          imports = [ ./modules ];
        };

        nixosModules.examples = {
          imports = [ ./examples ];
        };

      };
    };
}
