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
    home-manager.inputs.nixpkgs.follows = "nixpkgs";
    home-manager.url = "github:nix-community/home-manager";
  };

  outputs =
    inputs@{ flake-parts, self, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [
        inputs.devshell.flakeModule
        ./nix/devshells.nix
        ./nix/docs.nix
        ./nix/formatter.nix
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

          packages.default = self'.packages.artifacts;
          packages.artifacts-bin = pkgs.callPackage ./pkgs/artifacts { };
          packages.rust-docs = self'.packages.artifacts-bin.overrideAttrs (old: {
            name = "artifacts-rust-docs";
            buildPhase = ''
              echo "Building Rust API documentation..."
              cargo doc --lib --no-deps --document-private-items 2>&1 || {
                echo "Warning: Documentation build had errors but continuing"
              }
            '';
            installPhase = ''
              mkdir -p $out/share/doc/artifacts-rust
              cp -r target/doc/* $out/share/doc/artifacts-rust/

              cat > $out/share/doc/artifacts-rust/index.html <<'HTML'
              <!DOCTYPE html>
              <html>
              <head>
                <meta charset="utf-8">
                <meta http-equiv="refresh" content="0; URL=artifacts/index.html">
                <title>Artifacts API Documentation</title>
              </head>
              <body>
                <p>Redirecting to <a href="artifacts/index.html">artifacts/index.html</a>...</p>
              </body>
              </html>
              HTML

              echo "Documentation installed to $out/share/doc/artifacts-rust/"
            '';
            checkPhase = "true"; # Disable the regular test phase
          });

          packages.artifacts =
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
                  ${self'.packages.artifacts-bin}/bin/artifacts "$@"
                ''
              )
              {
                backends = {
                  test.nixos_check_serialization = pkgs.writers.writeBash "test_check" "exit 1"; # always fail
                  test.nixos_serialize = pkgs.writers.writeBash "test_serialize" ''
                    for file in "$out"/*; do
                        if [ -f "$file" ]; then
                            echo "=== Content of $file ==="
                            cat "$file"
                            echo "========================="
                        fi
                    done
                  '';
                  test.home_check_serialization = pkgs.writers.writeBash "test_home_check" "exit 1";
                  test.home_serialize = pkgs.writers.writeBash "test_home_serialize" ''
                    for file in "$out"/*; do
                        if [ -f "$file" ]; then
                            echo "=== Home Content of $file ==="
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
                  {
                    networking.hostName = name;
                    artifacts.default.backend.serialization = "test";

                    # Minimal configuration for flake check
                    system.stateVersion = "25.05";
                    boot.loader.grub.enable = false;
                    fileSystems."/" = {
                      device = "/dev/null";
                      fsType = "ext4";
                    };
                  }
                ];
              };
          in
          {
            machine-one = machineConfiguration "machine-one";
            machine-two = machineConfiguration "machine-two";
          };

        homeConfiguration.test = inputs.home-manager.lib.homeManagerConfiguration {
          pkgs = inputs.nixpkgs.legacyPackages."x86_64-linux".pkgs;
          modules = [
            self.homeModules.default
            self.homeModules.examples
            { artifacts.default.backend.serialization = "test"; }
            {
              home.stateVersion = "25.05";
              home.username = "test";
              home.homeDirectory = "/home/test";
            }
          ];
        };

        homeModules.default = {
          imports = [ ./modules/hm ];
        };
        homeModules.examples = {
          imports = [ ./examples/hm ];
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
