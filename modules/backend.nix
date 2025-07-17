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

  config = {

    # predefined backends
    artifacts.backend.passage = {
      # will be called on each artifact
      serialize = pkgs.writers.writeBash "serialize-with-passage" ''
        export PATH=${pkgs.passage}:$PATH
        for file in $(find "$out" -type f); do
            # Remove the $out prefix to get the relative path
            relative_path=''${file#$out/}
            echo "Serialize: $relative_path"
            cat "$file" | passage insert -m "artifacts/$relative_path"
        done
      '';
      deserialize = pkgs.writers.writeBash "deserialize-with-passage" ''
        export PATH=${pkgs.passage}:$PATH
        for file in $(find "$input" -type f); do
            # Remove the $input prefix to get the relative path
            relative_path=''${file#$input/}
            echo "Deserialize: $relative_path"
            passage show  "artifacts/$relative_path" > $out/$relative_path
        done
      '';
    };
  };

}
