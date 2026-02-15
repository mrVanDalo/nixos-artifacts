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

      # Artifacts with file backend for development
      packages.artifacts-with-file-backend = self'.packages.artifacts.override {
        backends = {
          test = {
            nixos_check_serialization = ../backends/file/check.sh;
            nixos_serialize = ../backends/file/serialize.sh;
            home_check_serialization = ../backends/file/check.sh;
            home_serialize = ../backends/file/serialize.sh;
            shared_check_serialization = ../backends/file/check.sh;
            shared_serialize = ../backends/file/shared-serialize.sh;
            capabilities = {
              shared = true;
              serializes = true;
            };
          };
        };
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
          self'.packages.artifacts-with-file-backend
        ];
      };
    };
}
