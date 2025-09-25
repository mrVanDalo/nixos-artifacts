{
  description = "scenario-simple";
  inputs.flake-parts.url = "github:hercules-ci/flake-parts";
  #inputs.nixos-artifacts.url = "path:///home/palo/dev/artifacts/nixos-artifacts";
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
              { lib, ... }:
              {
                options.artifacts.config.test.key = lib.mkOption {
                  type = lib.types.str;
                  default = "test";
                };
              }
            )
            (
              {
                pkgs,
                config,
                lib,
                ...
              }:
              {
                networking.hostName = "machine-name";
                artifacts.config.test.key = "example-key";
                artifacts.default.backend.serialization = "test";
                artifacts.store = {
                  test-artifact = {
                    files.very-simple-secrets = {
                      path = "/run/secrets/very-simple-secrets";
                    };
                    files.simple-secrets = {
                      path = "/run/secrets/simple-secrets";
                      owner = "deployer";
                      group = "deployer";
                    };
                    prompts.secret1.description = "secret number 1";
                    prompts.secret2.description = "secret number 2";
                    generator = pkgs.writers.writeBash "test_generator.sh" ''
                      cat $prompts/secret1 > $out/very-simple-secrets
                      cat $prompts/secret2 > $out/simple-secrets
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
