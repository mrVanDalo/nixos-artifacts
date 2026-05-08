{
  description = "Description for the project";

  inputs = {
    flake-parts.url = "github:hercules-ci/flake-parts";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs =
    inputs@{ flake-parts, self, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [
        inputs.flake-parts.flakeModules.partitions
        ./backends
        ./nix/options.nix
        ./nix/rust-docs.nix
        ./pkgs/artifacts
      ];

      partitionedAttrs = {
        apps = "dev";
        devShells = "dev";
        formatter = "dev";
        homeConfigurations = "dev";
      };

      partitions.dev.extraInputsFlake = ./dev;
      partitions.dev.module =
        { inputs, ... }:
        {
          imports = [
            inputs.devshell.flakeModule
            ./nix/devshells.nix
            ./nix/docs.nix
            ./nix/formatter.nix
          ];

          flake.homeConfigurations.test = inputs.home-manager.lib.homeManagerConfiguration {
            pkgs = inputs.nixpkgs.legacyPackages."x86_64-linux".pkgs;
            modules = [
              ./modules/hm
              ./examples/hm
              { artifacts.default.backend = "test"; }
              {
                home.stateVersion = "25.05";
                home.username = "test";
                home.homeDirectory = "/home/test";
              }
            ];
          };
        };
      perSystem =
        {
          self',
          system,
          ...
        }:
        {

          packages.default = self'.packages.artifacts;

          packages.artifacts = self.lib.mkArtifactCli {
            inherit system;
            backends = [ self'.packages.example-backend ];
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
                    artifacts.default.backend = "test";

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
