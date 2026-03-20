{ inputs, withSystem, ... }:
{
  imports = [ ./test/default.nix ];

  flake.lib = {
    mkBackend =
      {
        pkgs ? null,
        system ? null,
        name,
        nixos_check,
        nixos_serialize,
        home_check,
        home_serialize,
        shared_check ? null,
        shared_serialize ? null,
        settings ? { },
        nixos_enabled ? true,
        home_enabled ? true,
        shared_enabled ? true,
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

        nixosConfig = {
          enabled = nixos_enabled;
          check = "./nixos_check.sh";
          serialize = "./nixos_serialize.sh";
        };

        homeConfig = {
          enabled = home_enabled;
          check = "./home_check.sh";
          serialize = "./home_serialize.sh";
        };

        sharedConfig = lib.optionalAttrs (shared_check != null && shared_serialize != null) {
          enabled = shared_enabled;
          check = "./shared_check.sh";
          serialize = "./shared_serialize.sh";
        };

        backendToml = {
          ${name} = {
            nixos = nixosConfig;
            home = homeConfig;
          }
          // lib.optionalAttrs (shared_check != null && shared_serialize != null) {
            shared = sharedConfig;
          }
          // lib.optionalAttrs (settings != { }) {
            inherit settings;
          };
        };

        backendConfigFile = toml.generate "backend.toml" backendToml;
      in
      actualPkgs.runCommand "${name}-backend" { } ''
        mkdir -p $out

        cp ${backendConfigFile} $out/backend.toml

        cp ${nixos_check} $out/nixos_check.sh
        chmod +x $out/nixos_check.sh

        cp ${nixos_serialize} $out/nixos_serialize.sh
        chmod +x $out/nixos_serialize.sh

        cp ${home_check} $out/home_check.sh
        chmod +x $out/home_check.sh

        cp ${home_serialize} $out/home_serialize.sh
        chmod +x $out/home_serialize.sh

        ${lib.optionalString (shared_check != null) ''
          cp ${shared_check} $out/shared_check.sh
          chmod +x $out/shared_check.sh
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
