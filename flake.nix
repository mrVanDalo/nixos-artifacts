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
        ./flake-module.nix
      ];
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      perSystem =
        {
          config,
          self',
          inputs',
          pkgs,
          system,
          ...
        }:
        {
          # Per-system attributes can be defined here. The self' and inputs'
          # module parameters provide easy access to attributes of the same
          # system.

          # Equivalent to  inputs'.nixpkgs.legacyPackages.hello;
          packages.default = pkgs.hello;
        };
      flake = {

        nixosModules.default = {
          imports = [ ./modules ];
        };

        nixosConfigurations.example = inputs.nixpkgs.lib.nixosSystem {
          system = "x86_64-linux";
          modules = [
            self.nixosModules.default
            (
              { pkgs, config, ... }:
              {
                artifacts.default.backend = config.artifacts.backend.passage;
                artifacts.store.anotherTest = {
                  files.secret = { };
                  files.anotherSecret = { };
                  prompts.test = "test input";
                  prompts.something = "this is another file type";
                  generator = pkgs.writers.writeBash "test" ''
                    test
                  '';
                };
                artifacts.store.test = {
                  files.secret = { };
                  files.anotherSecret = { };
                  prompts.test = "test input";
                  prompts.something = "this is another file type";
                  generator = pkgs.writers.writeBash "test" ''
                    test
                  '';
                };
              }
            )
          ];
        };
      };
    };
}
