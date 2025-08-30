{ system }:
let
  #flake = builtins.getFlake (toString ./.);
  flake = builtins.getFlake (toString <flake>);
  #system = "x86_64-linux";
  pkgs = flake.inputs.nixpkgs.legacyPackages.${system};
  configurations = builtins.attrNames flake.nixosConfigurations;
  make = map (name: {
    machine = name;
    artifacts = flake.nixosConfigurations.${name}.config.artifacts.store;
  }) configurations;
in
pkgs.writeText "test.json" (builtins.toJSON make)
