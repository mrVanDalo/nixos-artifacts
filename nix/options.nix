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
            {
              cat <<'EOF'
            = NixOS options
            :description: Options exposed by the NixOS module (modules/store.nix)

            These options are exposed by `nixos-artifacts.nixosModules.default`
            and apply to `artifacts.store.<name>` declarations inside
            `nixosConfigurations`.

            NixOS-only options on this page that the Home Manager module does
            not expose:

            * xref:#_artifacts_store_name_shared[`shared`] — declare an
              artifact as shared across multiple NixOS machines.
            * `files.<name>.owner` / `files.<name>.group` — file ownership on
              the target system (Home Manager cannot set system-level
              ownership).

            For Home Manager, see
            xref:options-homemanager.adoc[Home Manager options].

            EOF
              cat ${nixosAsciidoc}
            } > docs/modules/ROOT/pages/options-nixos.adoc

            {
              cat <<'EOF'
            = Home Manager options
            :description: Options exposed by the Home Manager module (modules/hm/store.nix)

            These options are exposed by `nixos-artifacts.homeModules.default`
            and apply to `artifacts.store.<name>` declarations inside
            `homeConfigurations`.

            The Home Manager module intentionally does *not* expose:

            * `shared` — shared artifacts are a NixOS-only feature. HM
              artifacts are per-user and never aggregated across multiple
              targets.
            * `files.<name>.owner` / `files.<name>.group` — Home Manager
              cannot set system-level file ownership.

            For NixOS, see xref:options-nixos.adoc[NixOS options].

            EOF
              cat ${hmAsciidoc}
            } > docs/modules/ROOT/pages/options-homemanager.adoc
          '';
        };
      };
    };
}
