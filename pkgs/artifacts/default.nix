{ self, ... }:
{
  perSystem =
    {
      pkgs,
      self',
      lib,
      ...
    }:
    {
      packages.artifacts-bin = pkgs.rustPlatform.buildRustPackage rec {
        pname = "artifacts";
        version = "0.2.0";

        src = ./.;
        cargoLock.lockFile = ./Cargo.lock;

        doCheck = false;

        buildFeatures = [ "logging" ];

        cargoHash = lib.fakeSha256;

        meta = with lib; {
          description = "TUI for managing NixOS artifacts";
          homepage = "https://github.com/";
          license = licenses.mit;
          maintainers = [ ];
          platforms = platforms.unix;
          mainProgram = "artifacts";
        };
      };

    };
}
