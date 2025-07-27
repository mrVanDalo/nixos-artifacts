{
  pkgs,
  lib,
  ...
}:
with lib;
with types;
{

  # folder structure
  # ----------------
  #
  # $out/shared/<artifact-name>/<file-name>
  # $out/machines/<machine-name>/<artifact-name>/<file-name>
  #
  # $input/shared/<artifact-name>/<file-name>
  # $input/machines/<machine-name>/<artifact-name>/<file-name>
  #
  #
  # deserialization:
  # ----------------
  # $input -> program -> $out
  #
  # serialization:
  # --------------
  # $out -> program

  options = {

    artifacts.default.backend = {
      serialize = mkOption {
        type = package;
        description = "script for serialization";
      };
      deserialize = mkOption {
        type = package;
        description = "script for deserialization";
      };
    };

    artifacts.backend = mkOption {
      type = attrsOf (submodule {
        options = {
          serialize = mkOption {
            type = package;
            description = "script for serialization";
          };

          deserialize = mkOption {
            type = package;
            description = "script for deserialization";
          };
        };
      });
      default = { };
      description = "Backend configurations for artifact serialization and deserialization";
      example = literalExpression ''
        artifacts.backend.default = config.artifacts.backend.passage;
      '';
    };
  };

}
