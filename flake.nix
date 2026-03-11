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
        let
          mkBackend = import ./backends/default.nix { inherit lib pkgs; };
          testBackend = mkBackend "test" {
            nixos_check_serialization = ./backends/test/check.sh;
            nixos_serialize = ./backends/test/serialize.sh;
            home_check_serialization = ./backends/test/check.sh;
            home_serialize = ./backends/test/serialize.sh;
            shared_check_serialization = ./backends/test/check.sh;
            shared_serialize = ./backends/test/shared-serialize.sh;
            capabilities = {
              shared = true;
              serializes = true;
            };
          };
        in
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
            checkPhase = "true";
          });

          packages.artifacts =
            let
              mergeBackends = backendPaths:
                pkgs.runCommand "merged-backends.toml" { } ''
                  cat ${lib.concatStringsSep " " (map (p: "${p}/backend.toml") backendPaths)} > $out
                '';
              backendsFile = mergeBackends [ testBackend ];
            in
            pkgs.writers.writeBashBin "artifacts" ''
              set -e
              set -o pipefail
              export NIXOS_ARTIFACTS_BACKEND_CONFIG=${backendsFile}
              ${self'.packages.artifacts-bin}/bin/artifacts "$@"
            '';

          packages.artifacts-test = self'.packages.artifacts;
        };
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];

      flake = {
        lib.mkBackend = import ./backends/default.nix;

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
