{ lib, ... }:
with lib;
with types;
{
  options.artifacts.config = mkOption {
    type = attrsOf (
      attrsOf (oneOf [
        (listOf str)
        str
      ])
    );
    description = "Configuration attributes that can contain either strings or list of strings for each backend script";
    default = { };
  };
}
