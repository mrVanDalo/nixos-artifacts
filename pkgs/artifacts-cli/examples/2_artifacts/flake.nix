{
  description = "2-artifacts";
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
              { pkgs, ... }:
              {
                networking.hostName = "machine-name";
                artifacts.default.backend.serialization = "test";
                artifacts.store = {
                  artifact-one = {
                    files.first-secret = {
                      path = "/run/secrets/first-secret";
                    };
                    generator = pkgs.writers.writeBash "test_generator_one.sh" ''
                      echo "one" > "$out/first-secret"
                    '';
                  };
                  artifact-two = {
                    files.second-secret = {
                      path = "/run/secrets/second-secret";
                    };
                    generator = pkgs.writers.writeBash "test_generator_two.sh" ''
                      echo "two" > "$out/second-secret"
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
