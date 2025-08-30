{
  description = "Description for the project";

  inputs = {
    devshell.url = "github:numtide/devshell";
    flake-parts.url = "github:hercules-ci/flake-parts";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    treefmt-nix.inputs.nixpkgs.follows = "nixpkgs";
    treefmt-nix.url = "github:numtide/treefmt-nix";
  };

  outputs =
    inputs@{ flake-parts, self, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [
        ./nix/formatter.nix
        ./nix/devshells.nix
      ];
      perSystem =
        { pkgs, self', ... }:
        {
          packages.default = self'.packages.artifacts-cli;
          packages.artifacts-cli-bin = pkgs.callPackage ./pkgs/artifacts-tui { };

          packages.artifacts-cli =
            pkgs.callPackage
              (
                {
                  backend ? { },
                }:
                let
                  backendFile = (pkgs.formats.toml { }).generate "backend.toml" backend;
                in
                pkgs.writers.writeBashBin "artifacts" ''
                  set -e
                  set -o pipefail

                  MAKE=$(nix build --impure -I flake=$PWD --no-link --print-out-paths --expr 'import ${./make_file_generator.nix} { system = "${pkgs.system}"; }')
                  cat ${backendFile}
                  ${self'.packages.artifacts-cli-bin}/bin/artifacts-cli "$@" ${backendFile} ''${MAKE}
                ''
              )
              {
                backend = {
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
