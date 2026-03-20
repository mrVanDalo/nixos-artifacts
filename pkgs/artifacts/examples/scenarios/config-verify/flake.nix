{
  description = "config-verify-test";
  inputs.flake-parts.url = "github:hercules-ci/flake-parts";
  inputs.nixos-artifacts.url = "github:mrvandalo/nixos-artifacts";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    inputs@{ flake-parts, self, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [ "x86_64-linux" ];
      flake = {
        # NixOS machine with backend config
        nixosConfigurations.test-machine = inputs.nixpkgs.lib.nixosSystem {
          system = "x86_64-linux";
          modules = [
            inputs.nixos-artifacts.nixosModules.default
            (
              { pkgs, ... }:
              {
                networking.hostName = "test-machine";
                # Backend config that should appear in $config
                artifacts.default.backend.serialization = "test-config-verify";
                artifacts.store = {
                  # Regular NixOS artifact (will verify $config)
                  nixos-secret = {
                    files.secret-file = {
                      path = "/run/secrets/nixos-secret";
                    };
                    generator = pkgs.writers.writeBash "gen_nixos.sh" ''
                      echo "nixos-secret-value" > "$out/secret-file"
                    '';
                  };
                  # Shared artifact (will verify $machines/$users)
                  shared-config-secret = {
                    shared = true;
                    files.shared-file = {
                      path = "/run/secrets/shared-config";
                    };
                    generator = pkgs.writers.writeBash "gen_shared_config.sh" ''
                      echo "shared-config-value" > "$out/shared-file"
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
