{
  description = "circular-include";
  inputs.flake-parts.url = "github:hercules-ci/flake-parts";
  inputs.nixos-artifacts.url = "github:mrvandalo/nixos-artifacts";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    inputs@{ flake-parts, self, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [ "x86_64-linux" ];
      flake = {
        nixosConfigurations.machine-name = inputs.nixpkgs.lib.nixosSystem {
          system = "x86_64-linux";
          modules = [
            inputs.nixos-artifacts.nixosModules.default
            (
              {
                pkgs,
                config,
                lib,
                ...
              }:
              {
                networking.hostName = "machine-name";
                artifacts.default.backend.serialization = "test";
                artifacts.store = {
                  test-artifact = {
                    files.secret-file = {
                      path = "/run/secrets/secret-file";
                    };
                    generator = pkgs.writers.writeBash "test_generator.sh" ''
                      echo "secret content" > $out/secret-file
                    '';
                  };
                };
              }
            )
          ];
        };
      };
    };
}
