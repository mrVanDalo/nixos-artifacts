{
  description = "home-manager-only example with no nixosConfigurations";

  inputs.flake-parts.url = "github:hercules-ci/flake-parts";
  inputs.home-manager.inputs.nixpkgs.follows = "nixpkgs";
  inputs.home-manager.url = "github:nix-community/home-manager";
  inputs.nixos-artifacts.url = "github:mrvandalo/nixos-artifacts";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    inputs@{ flake-parts, self, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [ "x86_64-linux" ];
      flake = {
        homeConfigurations.test-user = inputs.home-manager.lib.homeManagerConfiguration {
          pkgs = inputs.nixpkgs.legacyPackages.x86_64-linux;
          modules = [
            inputs.nixos-artifacts.homeModules.default
            (
              { pkgs, ... }:
              {
                home.stateVersion = "25.05";
                home.username = "test-user";
                home.homeDirectory = "/home/test-user";
                artifacts.default.backend.serialization = "test";
                artifacts.store = {
                  home-secret = {
                    files.secret-file = { };
                    generator = pkgs.writers.writeBash "test_generator_home.sh" ''
                      echo "home-manager-only-secret" > "$out/secret-file"
                    '';
                  };
                  home-config = {
                    files.app-config = { };
                    generator = pkgs.writers.writeBash "test_generator_config.sh" ''
                      echo "setting=value" > "$out/app-config"
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