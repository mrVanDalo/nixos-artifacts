# generate options.adoc
{ self, inputs, ... }:
{
  perSystem =
    { pkgs, system, ... }:

    let
      # NixOS options
      nixosEval = import (inputs.nixpkgs + "/nixos/lib/eval-config.nix") {
        modules = [ ../modules ];
        inherit system;
      };
      nixosJson = (pkgs.nixosOptionsDoc { options = nixosEval.options.artifacts; }).optionsJSON;
      nixosFixedJSON = pkgs.runCommand "fix_nixos_json" { nativeBuildInputs = [ pkgs.gojq ]; } ''
        gojq 'del(.. | .declarations?)' ${nixosJson}/share/doc/nixos/options.json > $out
      '';
      nixosAsciidoc =
        pkgs.runCommand "nixos-options.adoc"
          {
            nativeBuildInputs = [ pkgs.nixos-render-docs ];
          }
          ''
            nixos-render-docs -j $NIX_BUILD_CORES options asciidoc \
              --manpage-urls ${pkgs.path + "/doc/manpage-urls.json"} \
              --revision "" \
              ${nixosFixedJSON} \
              $out
          '';

      # Home Manager options
      hmEval = pkgs.lib.evalModules {
        modules = [
          ../modules/hm
          { _module.check = false; }
        ];
        specialArgs = { inherit pkgs; };
      };
      hmJson = (pkgs.nixosOptionsDoc { options = hmEval.options.artifacts; }).optionsJSON;
      hmFixedJSON = pkgs.runCommand "fix_hm_json" { nativeBuildInputs = [ pkgs.gojq ]; } ''
        gojq 'del(.. | .declarations?)' ${hmJson}/share/doc/nixos/options.json > $out
      '';
      hmAsciidoc =
        pkgs.runCommand "hm-options.adoc"
          {
            nativeBuildInputs = [ pkgs.nixos-render-docs ];
          }
          ''
            nixos-render-docs -j $NIX_BUILD_CORES options asciidoc \
              --manpage-urls ${pkgs.path + "/doc/manpage-urls.json"} \
              --revision "" \
              ${hmFixedJSON} \
              $out
          '';
    in
    {
      apps.build-docs-options = {
        type = "app";
        program = pkgs.writeShellApplication {
          name = "eval-options-json";
          runtimeInputs = [ pkgs.coreutils ];
          text = ''
            cat ${nixosAsciidoc} > docs/modules/ROOT/pages/options-nixos.adoc
            cat ${hmAsciidoc} > docs/modules/ROOT/pages/options-homemanager.adoc
          '';
        };
      };
    };
}
