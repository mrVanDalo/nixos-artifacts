# generate options.adoc
{ self, inputs, ... }:
{
  perSystem =
    { pkgs, system, ... }:

    let
      eval = import (inputs.nixpkgs + "/nixos/lib/eval-config.nix") {
        modules = [ ../modules ];
        inherit system;
      };
      json = (pkgs.nixosOptionsDoc { options = eval.options.artifacts; }).optionsJSON;
      fixedJSON = pkgs.runCommand "fix_json" { nativeBuildInputs = [ pkgs.gojq ]; } ''
        gojq 'del(.. | .declarations?)' ${json}/share/doc/nixos/options.json > $out
      '';
      asciidoc = pkgs.runCommand "options.adoc" { nativeBuildInputs = [ pkgs.nixos-render-docs ]; } ''
        nixos-render-docs -j $NIX_BUILD_CORES options asciidoc \
          --manpage-urls ${pkgs.path + "/doc/manpage-urls.json"} \
          --revision "" \
          ${fixedJSON} \
          $out
      '';
    in
    {
      apps.build-docs-options = {
        type = "app";
        program = pkgs.writeShellApplication {
          name = "eval-options-json";
          runtimeInputs = [ pkgs.coreutils ];
          text = "cat ${asciidoc} > docs/modules/ROOT/pages/options.adoc";
        };
      };
      packages.asdf = asciidoc;
    };
}
