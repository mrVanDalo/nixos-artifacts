{

  artifacts.store.attic = {

    # represents a file on the target system
    files.env = { };

    # used to generate secrets based for rotation
    generator = {
      runtimeInputs = [ pkgs.openssl ];
      # todo maybe use writers here, with environment variables
      script = ''
        cat >"$out/env" <<EOF
        ATTIC_SERVER_TOKEN_RS256_SECRET_BASE64=$(openssl genrsa -traditional 4096 | base64 -w0)
        EOF
      '';
    };

  };

  # use artifact
  services.atticd = {
    enable = true;
    environmentFile = config.artifacts.store.attic.files.env.path;
  };

}
