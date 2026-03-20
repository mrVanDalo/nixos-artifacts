{
  description = "Development inputs for nixos-artifacts. These inputs are used by the dev partition but do not appear in consumers' lock files.";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    antora-flake.url = "github:mrvandalo/antora-flake";
    antora-flake.inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { ... }: { };
}
