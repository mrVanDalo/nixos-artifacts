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
        ./backends
        ./nix/devshells.nix
        ./nix/docs.nix
        ./nix/formatter.nix
        ./nix/options.nix
        ./nix/rust-docs.nix
        ./pkgs/artifacts
      ];
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
