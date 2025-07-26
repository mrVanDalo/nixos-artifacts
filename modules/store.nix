{ lib, config, ... }:
with lib;
with types;
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
              description = "The name of the artifact";
            };

            shared = mkOption {
              type = bool;
              default = false;
              description = "Whether the store is shared";
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
                        description = "The name of the filehandle";
                      };

                      path = mkOption {
                        type = str;
                        default = "/run/secrets/${artifactName}/${fileName}";
                        example = "/etc/ssh/ssh_host_ed25519_key";
                        description = "Path to the file on the target system.";
                      };

                      owner = mkOption {
                        type = str;
                        default = "root";
                        description = "owner of the file on the target system.";
                      };

                      group = mkOption {
                        type = str;
                        default = "root";
                        description = "group of the file on the target system.";
                      };
                    };
                  }
                )
              );
              default = { };
              description = "file definition on the target system";
            };

            prompts = mkOption {
              type = attrsOf (
                submodule (
                  { name, ... }:
                  let
                    promptName = name;
                  in
                  {
                    options = {
                      name = mkOption {
                        type = str;
                        default = promptName;
                        readOnly = true;
                        description = "The name of the prompt";
                      };

                      description = mkOption {
                        type = str;
                        description = "description shown during prompt entry";
                      };
                      type = mkOption {
                        type = enum [
                          "hidden"
                          "line"
                          "multiline"
                        ];
                        default = "line";
                        description = "Type of prompt input";
                      };
                    };
                  }
                )
              );
              default = { };
              description = "Prompts end up in $prompt/<name> in the generator script";
            };

            serialize = mkOption {
              type = package;
              default = config.artifacts.default.backend.serialize;
              description = "Serialization definition";
            };

            deserialize = mkOption {
              type = package;
              default = config.artifacts.default.backend.deserialize;
              description = "Deserialization definition";
            };

            generator = mkOption {
              type = nullOr package;
              default = null;
              description = ''
                Generator Script. Two environment variables are handed over to this script.
                - $prompt which is a folder containing files containing the prompt inputs (defined by the prompt option)
                - $out which is a folder the generator script must create a file for each file definition of the artifact.
              '';
              example = literalExpression ''
                pkgs.write.writeBash "random" ${"''"}
                  ${pkgs.xkcdpass}/bin/xkcdpass --numwords 10 > $out/random_password
                ${"''"};
              '';
            };
          };
        }
      )
    );
    default = { };
    description = "Artifacts store definitions";
  };
}
