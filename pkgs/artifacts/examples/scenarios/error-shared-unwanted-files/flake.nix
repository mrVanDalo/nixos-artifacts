{
  description = "shared-artifacts-unwanted-files-test";
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
                  # Shared artifact that creates wrong file (should error)
                  shared-secret = {
                    shared = true;
                    files.shared-key = {
                      path = "/run/secrets/shared-key";
                    };
                    generator = pkgs.writers.writeBash "gen_shared_wrong.sh" ''
                      # Creates wrong file instead of shared-key
                      echo "unwanted-value" > "$out/shared-unwanted"
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
                  # Same shared artifact creating wrong file
                  shared-secret = {
                    shared = true;
                    files.shared-key = {
                      path = "/run/secrets/shared-key";
                    };
                    generator = pkgs.writers.writeBash "gen_shared_wrong.sh" ''
                      # Creates wrong file instead of shared-key
                      echo "unwanted-value" > "$out/shared-unwanted"
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
