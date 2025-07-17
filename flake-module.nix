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

          storeArtifacts = nixosConfiguration.options.artifacts.store.value;

          generatorScripts = mapAttrsToList (
            artifactName:
            {
              files,
              prompts,
              generator,
              deserialize,
              serialize,
              ...
            }:
            ''

              echo generate ${artifactName}
              export out=$( mktemp -d )
              export input=$( mktemp -d )

              # create prompts
              ${concatStringsSep "" (
                mapAttrsToList (key: text: ''
                  echo "${text}"
                  read > $input/${key}
                '') prompts
              )}

              # try to deserialize
              ${deserialize}
              # todo check if all files are created
              # => output_is_ok=true

              if [[ $output_is_ok -eq false ]]; then
                ${generator}
                # todo check if all files are created
                ${serialize}
              fi

              # todo copy secrets to final $out/${artifactName}/<filename>

              # clean up
              rm -rf $out $input
            ''
          ) storeArtifacts;

        in
        concatStringsSep "\n" (flatten generatorScripts);

      asdf =
        {
          nixosConfiguration,
          ...
        }:
        pkgs.writers.writeBashBin "artifact-store" ''
          cat <<EOF
          ${allGeneratorScripts { inherit nixosConfiguration; }}
          EOF
        '';
    in
    {

      apps = {
        echo = {
          type = "app";
          program = asdf { nixosConfiguration = self.nixosConfigurations.example; };
        };
      };
    };

}
