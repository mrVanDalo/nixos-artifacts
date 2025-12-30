{
  description = "error-script-is-directory";
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
                  test-artifact = {
                    files.secret = {
                      path = "/run/secrets/secret";
                    };
                    generator = pkgs.writers.writeBash "generator.sh" ''
                      echo "test" > $out/secret
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
