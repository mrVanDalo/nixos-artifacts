{ pkgs, config, ... }:
{

  artifacts.store.passage = {

    # represents a file on the target system
    files.identities.path = "${config.home.homeDirectory}/.passage/identities";

    # used to generate secrets based for rotation
    generator = pkgs.writers.writeBash "generate-attic" ''
      ${pkgs.age}/bin/age-keygen > $out/identities
    '';
  };

}
