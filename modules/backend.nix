{
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

    artifacts.default.backend = mkOption {
      type = str;
      description = ''
        Name of the default backend used to (de)serialize artifacts.

        Every artifact whose `artifacts.store.<name>.backend` is not set
        explicitly falls back to this value. The name must match a backend
        provided via `mkArtifactCli`.
      '';
    };

  };

}
