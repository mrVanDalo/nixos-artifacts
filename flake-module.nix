{ self, inputs, ... }:
{
  imports = [ ];

  perSystem =
    {
      pkgs,
      self',
      lib,
      system,
      ...
    }:
    with lib;
    let
      nixosConfigurationsToVerify = filterAttrs (
        machine: configuration: builtins.hasAttr "artifacts" configuration.options
      ) self.nixosConfigurations;

      allGeneratorScripts =
        { nixosConfiguration, ... }:
        let

          promptCmd = {
            hidden = "read -sr prompt_value";
            line = "read -r prompt_value";
            multiline = ''
              echo 'press control-d to finish'
              prompt_value=$(cat)
            '';
          };

          storeArtifacts = nixosConfiguration.options.artifacts.store.value;

          stepPrompt =
            artifact:
            pkgs.writers.writeBash "prompt-${artifact.name}" ''
              ${lib.concatMapStringsSep "\n" (prompt: ''
                echo ${lib.escapeShellArg prompt.description}
                ${promptCmd.${prompt.type}}
                echo -n "$prompt_value" > "$prompts"/${prompt.name}
              '') (lib.attrValues artifact.prompts)}
            '';

          generatorScripts = map (artifact: ''
            echo "Prompt : ${artifact.name}" | boxes -d ansi-rounded

            prompts=$(mktemp -d)
            trap 'rm -rf $prompts' EXIT
            export prompts

            ${stepPrompt artifact}

            echo "Generating artifacts for ${artifact.name}"

            rm -rf $prompts
          '') (lib.attrValues storeArtifacts);

        in
        concatStringsSep "\n" (flatten generatorScripts);

      asdf =
        {
          nixosConfiguration,
          ...
        }:
        pkgs.writers.writeBashBin "artifact-store" ''
          export PATH=${
            lib.makeBinPath [
              pkgs.coreutils
              pkgs.boxes
            ]
          }
          ${allGeneratorScripts { inherit nixosConfiguration; }}
        '';
    in
    {

      apps = {
        default = {
          type = "app";
          program = asdf { nixosConfiguration = self.nixosConfigurations.example; };
        };
      };

      packages.default = asdf { nixosConfiguration = self.nixosConfigurations.example; };

    };

}
