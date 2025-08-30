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
                  map (name: { "'"''${name}"'" = configurations."'"''${name}"'".config.artifacts.store ;}) (builtins.attrNames configurations)
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

        nixosConfigurations.example = inputs.nixpkgs.lib.nixosSystem {
          system = "x86_64-linux";
          specialArgs = { inherit inputs; };
          modules = [
            self.nixosModules.default
            (
              { pkgs, config, ... }:
              {
                networking.hostName = "example";
                #artifacts.default.backend = config.artifacts.backend.agenix;
                #artifacts.config.agenix.storeDirAgain = ./secrets;
                #artifacts.config.agenix.publicHostKey = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIEUXkewyZ94A7CeCyVvN0KCqPn+8x1BZaGWMAojlfCXO";
                #artifacts.config.agenix.publicUserKeys = [
                #  "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAILE1jxUxvujFaj8kSjwJuNVRUinNuHsGeXUGVG6/lA1O"
                #];

                artifacts.default.backend.serialization = "test";

                artifacts.store = {
                  test = {
                    files.asdf = { };
                    generator = pkgs.writers.writeBash "generate-test" ''
                      echo "hallo" > $out/asdf
                    '';
                  };
                };
              }
            )
          ];
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
