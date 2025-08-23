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
      perSystem =
        { pkgs, self', ... }:
        {
          packages.default = self'.packages.artifacts-tui;
          packages.artifacts-tui = pkgs.callPackage ./pkgs/artifacts-tui { };
        };
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];

      flake = {

        flakeModules.default = ./flake-module.nix;

        nixosModules.default = {
          imports = [ ./modules ];
        };

        nixosModules.examples = {
          imports = [ ./examples ];
        };

      };
    };
}
