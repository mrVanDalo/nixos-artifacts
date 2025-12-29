let
  system = "x86_64-linux";
  filterAttrs =
    pred: set:
    builtins.removeAttrs set (builtins.filter (name: !pred name set.${name}) (builtins.attrNames set));
  flake = builtins.getFlake (toString <flake>);
  pkgs = flake.inputs.nixpkgs.legacyPackages.${system};
  nixosConfigurations = builtins.attrNames (
    filterAttrs (
      machine: configuration: builtins.hasAttr "artifacts" configuration.options
    ) flake.nixosConfigurations
  );
  homeConfigurations =
    let
      hc = if builtins.hasAttr "homeConfigurations" flake then flake.homeConfigurations else { };
    in
    builtins.attrNames (
      filterAttrs (user: configuration: builtins.hasAttr "artifacts" configuration.options) hc
    );
  nixos = map (name: {
    machine = name;
    artifacts = flake.nixosConfigurations.${name}.config.artifacts.store;
    config =
      if (builtins.hasAttr "config" flake.nixosConfigurations.${name}.config.artifacts) then
        flake.nixosConfigurations.${name}.config.artifacts.config
      else
        { };
  }) nixosConfigurations;
  home = map (name: {
    user = name;
    artifacts = flake.homeConfigurations.${name}.config.artifacts.store;
    config =
      if (builtins.hasAttr "config" flake.homeConfigurations.${name}.config.artifacts) then
        flake.homeConfigurations.${name}.config.artifacts.config
      else
        { };
  }) homeConfigurations;
  make = { inherit nixos home; };
in
pkgs.writeText "test.json" (builtins.toJSON make)
