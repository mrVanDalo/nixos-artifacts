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
              internal = true;
              description = "The name of the artifact";
            };

            description = mkOption {
              type = nullOr str;
              default = null;
              description = "Optional description of the artifact for documentation purposes.";
            };

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
                        defaultText = literalExpression "/run/artifacts/<artifact-name>/<file-name>";
                        default = "/run/artifacts/${artifactName}/${fileName}";
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
                        internal = true;
                        description = "The name of the prompt";
                      };

                      description = mkOption {
                        type = str;
                        default = "input for ${promptName}";
                        description = "description shown during prompt entry";
                      };

                    };
                  }
                )
              );
              default = { };
              description = "Prompts end up in $prompt/<name> in the generator script";
            };

            serialization = mkOption {
              type = str;
              default = config.artifacts.default.backend.serialization;
              defaultText = literalExpression "config.artifacts.default.backend.serialization";
              description = "Serialization definition";
            };

            generator = mkOption {
              type = nullOr package;
              default = null;
              description = ''
                Generator Script. These environment variables are handed over to this script.
                - `$out` a folder the generator script must create a file for each file definition of the artifact.
                - `$prompts` a folder containing files containing the prompt inputs (defined by the prompts option).
                - `$artifact` artifact name.
                - `$artifact_context` context type: "nixos", "homemanager", or "shared".
                - `$machine` machine name (only for NixOS targets).
                - `$username` username (only for Home Manager targets).
                - `$LOG_LEVEL` log level.
              '';
              example = literalExpression ''
                pkgs.write.writeBash "random" ${"''"}
                  ''${pkgs.xkcdpass}/bin/xkcdpass --numwords 10 > $out/random_password
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
