{ inputs, withSystem, ... }:
{
  imports = [ ./test/default.nix ];

  flake.lib = {
    mkBackend =
      {
        pkgs ? null,
        system ? null,
        name,
        nixos_check_serialization,
        nixos_serialize,
        home_check_serialization,
        home_serialize,
        shared_check_serialization ? null,
        shared_serialize ? null,
        settings ? { },
        capabilities ? {
          shared = false;
          serializes = true;
        },
        enabled ? true,
      }:
      let
        actualPkgs =
          if pkgs != null then
            pkgs
          else if system != null then
            inputs.nixpkgs.legacyPackages.${system}
          else
            throw "mkBackend requires either `pkgs` or `system`";
        inherit (actualPkgs) lib;
        toml = actualPkgs.formats.toml { };

        backendToml = {
          ${name} = {
            nixos_check_serialization = "./nixos_check_serialization.sh";
            nixos_serialize = "./nixos_serialize.sh";
            home_check_serialization = "./home_check_serialization.sh";
            home_serialize = "./home_serialize.sh";
          }
          // lib.optionalAttrs (shared_check_serialization != null) {
            shared_check_serialization = "./shared_check_serialization.sh";
          }
          // lib.optionalAttrs (shared_serialize != null) {
            shared_serialize = "./shared_serialize.sh";
          }
          // lib.optionalAttrs (settings != { }) {
            inherit settings;
          }
          // {
            inherit capabilities enabled;
          };
        };

        backendConfigFile = toml.generate "backend.toml" backendToml;
      in
      actualPkgs.runCommand "${name}-backend" { } ''
        mkdir -p $out

        cp ${backendConfigFile} $out/backend.toml

        cp ${nixos_check_serialization} $out/nixos_check_serialization.sh
        chmod +x $out/nixos_check_serialization.sh

        cp ${nixos_serialize} $out/nixos_serialize.sh
        chmod +x $out/nixos_serialize.sh

        cp ${home_check_serialization} $out/home_check_serialization.sh
        chmod +x $out/home_check_serialization.sh

        cp ${home_serialize} $out/home_serialize.sh
        chmod +x $out/home_serialize.sh

        ${lib.optionalString (shared_check_serialization != null) ''
          cp ${shared_check_serialization} $out/shared_check_serialization.sh
          chmod +x $out/shared_check_serialization.sh
        ''}

        ${lib.optionalString (shared_serialize != null) ''
          cp ${shared_serialize} $out/shared_serialize.sh
          chmod +x $out/shared_serialize.sh
        ''}
      '';

    mkArtifactCli =
      { system, backends }:
      withSystem system (
        { pkgs, self', ... }:
        let
          lib = pkgs.lib;
          backendPaths = map (backend: "\"${backend}/backend.toml\"") backends;
          backendsFile = pkgs.runCommand "merged-backends.toml" { } ''
            cat > $out <<EOF
            include = [${lib.concatStringsSep ", " backendPaths}]
            EOF
          '';
        in
        pkgs.writers.writeBashBin "artifacts" ''
          set -e
          set -o pipefail
          export NIXOS_ARTIFACTS_BACKEND_CONFIG=${backendsFile}
          ${self'.packages.artifacts-bin}/bin/artifacts "$@"
        ''
      );
  };
}
