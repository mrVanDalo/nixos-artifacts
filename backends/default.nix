{ inputs, withSystem, ... }:
{
  imports = [ ./test/default.nix ];

  flake.lib = {
    # creates a backend package for the artifacts system.
    #
    # Parameters (excluding pkgs and system):
    #
    #   name :: string
    #     Backend identifier used in artifact declarations
    #     (artifacts.store.<name>.serialization = "<name>")
    #
    #   settings :: attribute set (default: { })
    #     Backend-specific configuration passed to scripts via environment.
    #     Arbitrary key-value pairs.
    #
    #   home_enabled :: bool (default: true)
    #     Whether the backend is active for homeConfigurations.
    #
    #   home_check :: path
    #     Script to check if serialization is needed for home-manager artifacts.
    #     Exit 0 = up-to-date, non-zero = needs regeneration.
    #
    #   home_serialize :: path
    #     Script to serialize home-manager artifact files to backend storage.
    #
    #   nixos_enabled :: bool (default: true)
    #     Whether the backend is active for nixosConfigurations.
    #
    #   nixos_check :: path
    #     Script to check if serialization is needed for NixOS artifacts.
    #     Exit 0 = up-to-date, non-zero = needs regeneration.
    #
    #   nixos_serialize :: path
    #     Script to serialize NixOS artifact files to backend storage.
    #
    #   shared_enabled :: bool (default: true)
    #     Whether the backend handles shared artifacts (multi-machine).
    #
    #   shared_check :: path? (default: null)
    #     Script to check serialization for shared artifacts.
    #     Must pair with shared_serialize.
    #
    #   shared_serialize :: path? (default: null)
    #     Script to serialize shared artifacts.
    #     Must pair with shared_check.
    #
    # Notes:
    #   - shared_check and shared_serialize must both be provided or both be null
    #   - Scripts are copied to output and made executable
    #   - Output is a directory with backend.toml and all script files
    #
    mkBackend =
      {
        pkgs ? null,
        system ? null,
        name,
        settings ? { },
        home_enabled ? true,
        home_check,
        home_serialize,
        nixos_enabled ? true,
        nixos_check,
        nixos_serialize,
        shared_enabled ? true,
        shared_check ? null,
        shared_serialize ? null,
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

    # creates a CLI wrapper package for the artifacts command.
    #
    # This function generates a bash script that wraps the artifacts-bin binary
    # with the proper backend configuration. The wrapper sets the
    # NIXOS_ARTIFACTS_BACKEND_CONFIG environment variable to point to a merged
    # backend configuration file.
    #
    # Parameters:
    #
    #   system :: string
    #     The system architecture (e.g., "x86_64-linux")
    #
    #   backends :: list of paths
    #     List of backend package paths. Each path should point to a directory
    #     containing a backend.toml file (as produced by mkBackend).
    #
    # Output:
    #   A bash script named "artifacts" that:
    #     - Sets NIXOS_ARTIFACTS_BACKEND_CONFIG to a generated config file
    #     - The config file uses the include directive to merge all backend.toml files
    #     - Delegates to the artifacts-bin binary with all arguments
    #
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
