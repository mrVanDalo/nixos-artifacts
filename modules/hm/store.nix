{ lib, config, ... }:
with lib;
with types;
let
  commonOptions = import ../common-store-options.nix { inherit lib config; };
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

            files = mkOption {
              type = attrsOf (
                submodule (
                  { name, ... }:
                  let
                    fileName = name;
                  in
                  {
                    options = {

                      name = mkOption {
                        type = str;
                        default = fileName;
                        readOnly = true;
                        internal = true;
                        description = "The name of the filehandle";
                      };

                      path = mkOption {
                        type = str;
                        defaultText = literalExpression "\${XDG_RUNTIME_DIR}/artifacts/<artifact-name>/<file-name>";
                        default = "\${XDG_RUNTIME_DIR}/artifacts/${artifactName}/${fileName}";
                        example = literalExpression "\${XDG_CONFIG_DIR}/artifacts/${artifactName}/${fileName}";
                        description = "Path to the file on the target system.";
                      };

                      mode = mkOption {
                        type = types.str;
                        default = "0400";
                        description = ''
                          Permissions mode of the decrypted secret in a format understood by chmod.
                        '';
                      };

                    };
                  }
                )
              );
              default = { };
              description = "file definition on the target system";
            };

          }
          // commonOptions;
        }
      )
    );
    default = { };
    description = "Artifacts store definitions";
  };
}
