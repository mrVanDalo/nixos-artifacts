{
  description = "test-bubblewrap-network-blocking";
  inputs.flake-parts.url = "github:hercules-ci/flake-parts";
  inputs.nixos-artifacts.url = "path:/var/tmp/vibe-kanban/worktrees/0125-finish-improve-t/nixos-artifacts";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    inputs@{ flake-parts, self, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [ "x86_64-linux" ];
      flake = {
        nixosConfigurations."machine-one" = inputs.nixpkgs.lib.nixosSystem {
          system = "x86_64-linux";
          modules = [
            inputs.nixos-artifacts.nixosModules.default
            (
              { pkgs, ... }:
              {
                networking.hostName = "machine-one";
                artifacts.default.backend.serialization = "test";
                artifacts.store = {
                  test-network-block = {
                    files.test-file = {
                      path = "/run/secrets/test-network-block";
                    };
                    generator = pkgs.writers.writeBash "network_test_generator.sh" ''
                      echo "Attempting to make network call - should be blocked by bubblewrap..."
                      echo "If this succeeds, bubblewrap network isolation is not working properly"

                      # Try to make a network call - this should fail due to bubblewrap blocking
                      if curl -s --max-time 5 https://1.1.1.1/ > /dev/null 2>&1; then
                        # Network call succeeded (this shouldn't happen with bubblewrap)
                        echo "generated-content" > $out/test-file
                        exit 0
                      else
                        # Network call failed as expected due to bubblewrap blocking
                        echo "ERROR: Network call blocked by bubblewrap as expected - this is the test failure case" >&2
                        exit 1
                      fi
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
