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
      serialization = mkOption {
        type = str;
        description = "script for serialization";
      };
    };

  };

}
