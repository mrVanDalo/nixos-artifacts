{ lib, config, ... }:
with lib;
with types;
let
  common = import ../common-store-options.nix { inherit lib config; };
in
{
  options.artifacts.store = mkOption {
    type = attrsOf (
      submodule (
        { name, ... }:
        let
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

            # Home Manager stores intentionally omit `shared` (see
            # ../store.nix): HM artifacts are per-user and never aggregated
            # across multiple targets.

            files = mkOption {
              type = attrsOf (
                submodule (
                  { name, ... }:
                  let
                    fileName = name;
                  in
                  {
                    options = (common.mkCommonFileOptions { inherit fileName; }) // {

                      path = mkOption {
                        type = str;
                        defaultText = literalExpression "\${XDG_RUNTIME_DIR}/artifacts/<artifact-name>/<file-name>";
                        default = "\${XDG_RUNTIME_DIR}/artifacts/${artifactName}/${fileName}";
                        example = literalExpression "\${XDG_CONFIG_DIR}/artifacts/${artifactName}/${fileName}";
                        description = "Path to the file on the target system.";
                      };

                      # Home Manager stores intentionally omit `owner` and
                      # `group` (see ../store.nix): home-manager cannot set
                      # system-level file ownership.

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
