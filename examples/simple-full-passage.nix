{

  nixosConfiguration.attic = {

    # folder structure
    # ----------------
    #
    # $out/shared/<artifact-name>/<file-name>
    # $out/machines/<machine-name>/<artifact-name>/<file-name>
    #
    # $input/shared/<artifact-name>/<file-name>
    # $input/machines/<machine-name>/<artifact-name>/<file-name>
    #
    #
    # deserialization:
    # ----------------
    # $input -> program -> $out
    #
    # serialization:
    # --------------
    # $out -> program

    # default for all artifacts
    artifacts.config.backend.default = config.artifacts.config.backend.passage;

    # predefined backends
    artifacts.config.backend.passage = {
      serialize = {
        runtimeInputs = [ pkgs.passage ];
        script = ''
          # Find all files and process them
          for file in $(find "$out" -type f); do
              # Remove the $out prefix to get the relative path
              relative_path=''${file#$out/}
              echo "Serialize: $relative_path"
              cat "$file" | passage insert -m "artifacts/$relative_path"
          done
        '';
      };
      deserialize = {
        runtimeInputs = [ pkgs.passage ];
        script = ''
          for file in $(find "$input" -type f); do
              # Remove the $input prefix to get the relative path
              relative_path=''${file#$input/}
              echo "Deserialize: $relative_path"
              passage show  "artifacts/$relative_path" > $out/$relative_path
          done
        '';
      };
    };

    artifacts.store.attic = {

      # represents a file on the target system
      files.env = {
        owner = "atticd";
        group = "atticd";
        path = "/var/lib/attic/secrets/env";
      };

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

  };
}
