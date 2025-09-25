{
  description = "artifact-names";
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
                  "artifact.one" = {
                    files."first.secret".path = "/run/secrets/first.secret";
                    files."second_secret".path = "/run/secrets/second_secret";
                    files."third-secret".path = "/run/secrets/third-secret";
                    files."forthSecret".path = "/run/secrets/forthSecret";
                    files."fifth secret".path = "/run/secrets/fifth secret";
                    generator = pkgs.writers.writeBash "test_generator.sh" ''
                      echo "test" > "$out/first.secret"
                      echo "test" > "$out/second_secret"
                      echo "test" > "$out/third-secret"
                      echo "test" > "$out/forthSecret"
                      echo "test" > "$out/fifth secret"
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
