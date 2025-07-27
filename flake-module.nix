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

          stepCheckOutput =
            artifact:
            pkgs.writers.writeBash "check-output-${artifact.name}" ''
              all_files_missing=true
              all_files_present=true
              ${lib.concatMapStringsSep "\n" (file: ''
                if test -e $out/${lib.escapeShellArg file.name} ; then
                  all_files_missing=false
                else
                  all_files_present=false
                fi
              '') (lib.attrValues artifact.files)}

              if [ $all_files_present = true ]; then
                echo "All artifacts for ${artifact.name} are present"
                exit 0
              fi

              if [ $all_files_missing = true ]; then
                echo "No artifacts for ${artifact.name} are present"
                exit 1
              fi

              echo "Inconsistent state for generator: ${artifact.name}"
              exit 2
            '';

          stepDeserialize =
            artifact:
            pkgs.writers.writeBash "deserialize-${artifact.name}" ''
              export input=$(mktemp -d)
              trap 'rm -rf $input' EXIT

              ${lib.concatMapStringsSep "\n" (file: "touch $input/${lib.escapeShellArg file.name}") (
                lib.attrValues artifact.files
              )}
              # run deserialization script
              ${artifact.deserialize}
              # check if deserialisation went well
              ${stepCheckOutput artifact}
              exit_code=$?
              if [ $exit_code -eq 0 ]; then
                  exit 0
              elif [ $exit_code -ne 1 ]; then
                  exit 2
              fi
              exit 1
            '';

          artifactSteps =
            artifact:
            pkgs.writers.writeBash "artifact-${machineName}-${artifact.name}" ''
              export artifact=${artifact.name}
              export machine=${machineName}

              original_final_output="$final_output"
              unset final_output

              export out=$(mktemp -d)
              trap 'rm -rf $out' EXIT

              ${stepDeserialize artifact}
              exit_code=$?
              if [ $exit_code -eq 0 ]; then
                  mkdir -p $original_final_output/per-machine/${machineName}/${artifact.name}/
                  cp -r $out/* $original_final_output/per-machine/${machineName}/${artifact.name}/
                  exit 0
              elif [ $exit_code -ne 1 ]; then
                  exit 2
              fi

              # cleanup
              rm -rf $out
              unset $out

              echo "Prompt : ${artifact.name}" | boxes -d ansi-rounded

              export prompts=$(mktemp -d)
              trap 'rm -rf $prompts' EXIT

              ${stepPrompt artifact}

              export out=$(mktemp -d)
              trap 'rm -rf $prompts $out' EXIT

              echo "Generating artifacts for ${artifact.name}"
              ${artifact.generator}

              ${stepCheckOutput artifact}
              exit_code=$?
              if [ $exit_code -eq 0 ]; then
                ${artifact.serialize}
                exit_code=$?
                if [ $exit_code -ne 0 ]; then
                    exit 2
                fi

                mkdir -p $original_final_output/per-machine/${machineName}/${artifact.name}/
                cp -r $out/* $original_final_output/per-machine/${machineName}/${artifact.name}/
                exit 0
              elif [ $exit_code -ne 0 ]; then
                exit 2
              fi
            '';

          generatorScripts = map artifactSteps (lib.attrValues storeArtifacts);

        in
        concatStringsSep "\n echo exit=$?\n" (flatten generatorScripts);

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

          set -e
          export final_output=$(mktemp -d)
          echo "final_output=$final_output"
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
