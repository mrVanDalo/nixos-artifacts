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

      artifactScript =
        { nixosConfiguration, machineName, ... }:
        let

          storeArtifacts = nixosConfiguration.options.artifacts.store.value;

          promptCmd = {
            hidden = "read -sr prompt_value";
            line = "read -r prompt_value";
            multiline = ''
              echo 'press control-d to finish'
              prompt_value=$(cat)
            '';
          };

          stepPrompt =
            artifact:
            pkgs.writers.writeBash "prompt-${artifact.name}" ''
              ${lib.concatMapStringsSep "\n" (prompt: ''
                echo ${lib.escapeShellArg prompt.description}
                ${promptCmd.${prompt.type}}
                echo -n "$prompt_value" > "$prompts"/${prompt.name}
              '') (lib.attrValues artifact.prompts)}
            '';

          artifactSteps =
            artifact:
            pkgs.writers.writeBash "artifact-${machineName}-${artifact.name}" ''
              echo "Prompt : ${artifact.name}" | boxes -d ansi-rounded

              export artifact=${artifact.name}
              export machine=${machineName}

              prompts=$(mktemp -d)
              echo "prompts=$prompts"
              trap 'rm -rf $prompts' EXIT
              export prompts

              ${stepPrompt artifact}

              out=$(mktemp -d)
              echo "out=$out"
              trap 'rm -rf $prompts $out' EXIT
              export out

              echo "Generating artifacts for ${artifact.name}"
              ${artifact.generator}

              echo "todo: check if outputs are all there"

              ${artifact.serialize}
              mkdir -p /tmp/artifacts/per-machine/${machineName}/${artifact.name}/
              cp -r $out/* /tmp/artifacts/per-machine/${machineName}/${artifact.name}/

              rm -rf $prompts
            '';

          generatorScripts = map artifactSteps (lib.attrValues storeArtifacts);

        in
        concatStringsSep "\n" (flatten generatorScripts);

      asdf =
        {
          nixosConfiguration,
          machineName,
          ...
        }:
        pkgs.writers.writeBashBin "artifact-store" ''
          export PATH=${
            lib.makeBinPath [
              pkgs.coreutils
              pkgs.boxes
              pkgs.findutils
            ]
          }
          mkdir -p /tmp/artifacts
          ${artifactScript { inherit nixosConfiguration machineName; }}
        '';
    in
    {

      apps = {
        default = {
          type = "app";
          program = asdf {
            nixosConfiguration = self.nixosConfigurations.example;
            machineName = "example";
          };
        };
      };

      packages.default = asdf {
        nixosConfiguration = self.nixosConfigurations.example;
        machineName = "example";
      };

    };

}
