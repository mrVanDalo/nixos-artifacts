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
        #./flake-module.nix
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
                  nix eval --json .#nixosConfigurations --apply "
                  configurations:
                  map (name: { machine = "'"''${name}"'" ; artifacts = configurations."'"''${name}"'".config.artifacts.store ;}) (builtins.attrNames configurations)
                  " | ${pkgs.gojq}/bin/gojq

                  #${self'.packages.artifacts-cli-bin}/bin/artifacts-cli ${backendFile} "$@"
                  cat ${backendFile}
                ''
              )
              {
                backend = {
                  test.check_serialization = "./test_check.sh";
                  test.serialize = "./test_serialize.sh";
                  test.deserialize = "./test_deseraialize.sh";
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
