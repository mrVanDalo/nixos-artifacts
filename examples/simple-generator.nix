{ pkgs, config, ... }:
{

  artifacts.store.attic = {

    # represents a file on the target system
    files.env = {
      owner = "atticd";
      group = "atticd";
      path = "/var/lib/attic/secrets/env";
    };

    # used to generate secrets based for rotation
    generator = pkgs.writers.writeBash "generate-attic" ''
      cat >"$out/env" <<EOF
      ATTIC_SERVER_TOKEN_RS256_SECRET_BASE64=$(${pkgs.openssl}/bin/openssl genrsa -traditional 4096 | base64 -w0)
      EOF
    '';
  };

  # use artifact
  services.atticd = {
    enable = true;
    environmentFile = config.artifacts.store.attic.files.env.path;
  };

}
