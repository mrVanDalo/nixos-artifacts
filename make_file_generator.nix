{ system }:
let
  filterAttrs =
    pred: set:
    builtins.removeAttrs set (builtins.filter (name: !pred name set.${name}) (builtins.attrNames set));
  flake = builtins.getFlake (toString <flake>);
  pkgs = flake.inputs.nixpkgs.legacyPackages.${system};
  configurations = builtins.attrNames (
    filterAttrs (
      machine: configuration: builtins.hasAttr "artifacts" configuration.options
    ) flake.nixosConfigurations
  );
  make = map (name: {
    machine = name;
    artifacts = flake.nixosConfigurations.${name}.config.artifacts.store;
    config =
      if (builtins.hasAttr "config" flake.nixosConfigurations.${name}.config.artifacts) then
        flake.nixosConfigurations.${name}.config.artifacts.config
      else
        { };
  }) configurations;
in
pkgs.writeText "test.json" (builtins.toJSON make)
