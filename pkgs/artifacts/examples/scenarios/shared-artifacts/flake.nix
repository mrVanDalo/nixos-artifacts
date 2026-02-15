{
  description = "shared-artifacts-test";
  inputs.flake-parts.url = "github:hercules-ci/flake-parts";
  inputs.nixos-artifacts.url = "github:mrvandalo/nixos-artifacts";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    inputs@{ flake-parts, self, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [ "x86_64-linux" ];
      flake = {
        nixosConfigurations.machine-one = inputs.nixpkgs.lib.nixosSystem {
          system = "x86_64-linux";
          modules = [
            inputs.nixos-artifacts.nixosModules.default
            (
              { pkgs, ... }:
              {
                networking.hostName = "machine-one";
                artifacts.default.backend.serialization = "test";
                artifacts.store = {
                  # Shared artifact across both machines
                  shared-secret = {
                    shared = true;
                    files.shared-key = {
                      path = "/run/secrets/shared-key";
                    };
                    generator = pkgs.writers.writeBash "gen_shared.sh" ''
                      echo "shared-value" > "$out/shared-key"
                    '';
                  };
                  # Machine-specific artifact
                  local-secret = {
                    files.local-key = {
                      path = "/run/secrets/local-key";
                    };
                    generator = pkgs.writers.writeBash "gen_local_one.sh" ''
                      echo "local-one" > "$out/local-key"
                    '';
                  };
                };
              }
            )
          ];
        };
        nixosConfigurations.machine-two = inputs.nixpkgs.lib.nixosSystem {
          system = "x86_64-linux";
          modules = [
            inputs.nixos-artifacts.nixosModules.default
            (
              { pkgs, ... }:
              {
                networking.hostName = "machine-two";
                artifacts.default.backend.serialization = "test";
                artifacts.store = {
                  # Same shared artifact
                  shared-secret = {
                    shared = true;
                    files.shared-key = {
                      path = "/run/secrets/shared-key";
                    };
                    generator = pkgs.writers.writeBash "gen_shared.sh" ''
                      echo "shared-value" > "$out/shared-key"
                    '';
                  };
                  # Different machine-specific artifact
                  local-secret = {
                    files.local-key = {
                      path = "/run/secrets/local-key";
                    };
                    generator = pkgs.writers.writeBash "gen_local_two.sh" ''
                      echo "local-two" > "$out/local-key"
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
