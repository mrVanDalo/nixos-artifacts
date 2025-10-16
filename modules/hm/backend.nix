{
  pkgs,
  lib,
  ...
}:
with lib;
with types;
{
  options = {

    artifacts.default.backend = {
      serialization = mkOption {
        type = str;
        description = "script for serialization";
      };
    };

  };

}
