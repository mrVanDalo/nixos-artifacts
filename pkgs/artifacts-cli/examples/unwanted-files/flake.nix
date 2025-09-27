{
  description = "generate-wrong";
  inputs.flake-parts.url = "github:hercules-ci/flake-parts";
  inputs.nixos-artifacts.url = "github:mrvandalo/nixos-artifacts";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    inputs@{ flake-parts, self, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [ "x86_64-linux" ];
      flake = {
        nixosConfigurations."unwanted-files" = inputs.nixpkgs.lib.nixosSystem {
          system = "x86_64-linux";
          modules = [
            inputs.nixos-artifacts.nixosModules.default
            (
              { pkgs, ... }:
              {
                networking.hostName = "machine-name";
                artifacts.default.backend.serialization = "test";
                artifacts.store = {
                  test-artifact = {
                    files = {
                      very-simple-secrets = {
                        path = "/run/secrets/very-simple-secrets";
                      };
                      simple-secrets = {
                        path = "/run/secrets/simple-secrets";
                        owner = "deployer";
                        group = "deployer";
                      };
                    };
                    generator = pkgs.writers.writeBash "test_generator_unwanted_files.sh" ''
                      # this actually is right
                      echo "test" > "$out/very-simple-secrets"
                      echo "test" > "$out/simple-secrets"
                      # this file should not exist
                      touch "$out/should_not_be_there"
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
