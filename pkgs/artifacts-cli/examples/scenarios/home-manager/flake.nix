{
  description = "simple home manager example";

  inputs.flake-parts.url = "github:hercules-ci/flake-parts";
  inputs.home-manager.inputs.nixpkgs.follows = "nixpkgs";
  inputs.home-manager.url = "github:nix-community/home-manager";
  inputs.nixos-artifacts.url = "github:mrvandalo/nixos-artifacts/home-manager";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    inputs@{ flake-parts, self, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [ "x86_64-linux" ];
      flake = {
        homeConfigurations.test = inputs.home-manager.lib.homeManagerConfiguration {
          pkgs = inputs.nixpkgs.legacyPackages.x86_64-linux;
          modules = [
            inputs.nixos-artifacts.homeModules.default
            (
              { pkgs, ... }:
              {
                home.stateVersion = "25.05";
                home.username = "some-test-name";
                home.homeDirectory = "/home/test";
                artifacts.default.backend.serialization = "test";
                artifacts.store = {
                  artifact-one = {
                    files.first-secret = { };
                    generator = pkgs.writers.writeBash "test_generator_one.sh" ''
                      echo "one" > "$out/first-secret"
                    '';
                  };
                  artifact-two = {
                    files.second-secret = { };
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
