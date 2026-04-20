{ lib, config, ... }:
with lib;
with types;
let
  common = import ./common-store-options.nix { inherit lib config; };
in
{
  options.artifacts.store = mkOption {
    type = attrsOf (
      submodule (
        { name, ... }:
        let
          # bind outer name before inner submodule shadows it
          artifactName = name;
        in
        {
          options = {

            name = mkOption {
              type = str;
              default = artifactName;
              readOnly = true;
              internal = true;
              description = "The name of the artifact";
            };

            description = mkOption {
              type = nullOr str;
              default = null;
              description = "Optional description of the artifact for documentation purposes.";
            };

            # NixOS-only: artifacts may be shared across machines. Home Manager
            # stores omit this because they are per-user and never aggregated.
            shared = mkOption {
              type = bool;
              default = false;
              description = ''
                Whether this artifact is shared across multiple machines and/or home-manager configurations.

                When `true`, the artifact is generated once and distributed to all targets that define it.
                All definitions with the same artifact name and `shared = true` are aggregated together.

                The backend must provide a `shared.check` and `shared.serialize` scripts to handle shared artifacts.
                The scripts receive a unified `$targets` environment variable pointing to a JSON file containing
                all target names, types, and their respective backend configurations.
              '';
            };

            files = mkOption {
              type = attrsOf (
                submodule (
                  { name, ... }:
                  let
                    # bind outer name before inner submodule shadows it
                    fileName = name;
                  in
                  {
                    options = (common.mkCommonFileOptions { inherit fileName; }) // {

                      path = mkOption {
                        type = str;
                        defaultText = literalExpression "/run/artifacts/<artifact-name>/<file-name>";
                        default = "/run/artifacts/${artifactName}/${fileName}";
                        example = "/etc/ssh/ssh_host_ed25519_key";
                        description = "Path to the file on the target system.";
                      };

                      # NixOS-only: owner and group are managed by the system.
                      # Home Manager stores omit these because home-manager
                      # cannot change system-level file ownership.
                      owner = mkOption {
                        type = str;
                        default = "root";
                        description = "Owner of the file on the target system.";
                      };

                      group = mkOption {
                        type = str;
                        default = "root";
                        description = "Group of the file on the target system.";
                      };

                    };
                  }
                )
              );
              default = { };
              description = "file definition on the target system";
            };

          }
          // common.artifactOptions;
        }
      )
    );
    default = { };
    description = "Artifacts store definitions";
  };
}
