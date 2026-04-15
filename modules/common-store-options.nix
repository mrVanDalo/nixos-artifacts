{ lib, config }:
with lib;
with types;
{

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
    description = ''
      Name of the backend used to serialize this artifact.

      Defaults to `artifacts.default.backend.serialization`. Set this to
      select a different backend for this specific artifact, while other
      artifacts continue to use the default. Backends are configured via
      `mkArtifactCli` and referenced here by the `name` passed to `mkBackend`.
    '';
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

}
