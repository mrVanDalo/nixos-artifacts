{ inputs, lib, ... }:
{
  perSystem =
    {
      pkgs,
      self',
      system,
      ...
    }:
    {
      # allow unfree packages
      _module.args.pkgs = import inputs.nixpkgs {
        inherit system;
        config.allowUnfree = true;
      };

      devshells.default = {
        env = [
          {
            name = "NIXOS_ARTIFACTS_PROJECT_ROOT";
            eval = "\${NIXOS_ARTIFACTS_PROJECT_ROOT:-$(pwd)}";
          }
        ];

        packages = [
          self'.formatter
          self'.packages.artifacts
        ];
      };
    };
}
