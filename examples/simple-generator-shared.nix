{

  nixosConfigurations.iam = {

    artifacts.store.oidc_git = {

      # use the same secret in all nixosConfigurations
      shared = true;

      # represents a file on the target system
      # the value of this file can differ on each system
      files.oidc_secret = {
        owner = "kanidm";
        group = "kanidm";
        path = "/var/lib/kanidm/secrets/git/oidc_secret";
      };

      # used to generate secrets based for rotation
      generator = {
        runtimeInputs = [ pkgs.pwgen ];
        # todo maybe use writers here, with environment variables
        script = ''
          pwgen -s 48 1 > "$out/oidc_secret"
        '';
      };

    };

    services.kanidm.enableServer = true;
    services.kanidm.provision = {
      enable = true;
      persons."palo" = {
        displayName = "Ingolf Wagner";
        legalName = "Ingolf Wagner";
        groups = lib.attrNames config.services.kanidm.provision.groups;
      };
      groups."git_users" = { };
      systems.oauth2.git = {
        displayName = "git";
        originUrl = "https://git.example.com/user/oauth2/kanidm/callback";
        originLanding = "https://git.example.com/";
        preferShortUsername = true;
        scopeMaps."git_users" = [
          "openid"
          "email"
          "profile"
        ];
        basicSecretFile = config.artifacts.store.oidc_git.files.oidc_secret.path;
      };
    };

  };

  nixosConfigurations.git = {

    artifacts.store.oidc_git = {

      # use the same secret in all nixosConfigurations
      shared = true;

      # represents a file on the target system
      # the value of this file can differ on each system
      files.oidc_secret = {
        owner = "forgejo";
        group = "forgejo";
        path = "/var/lib/forgejo/secrets/kanidm/oidc_secret";
      };

      # used to generate secrets based for rotation
      generator = {
        runtimeInputs = [ pkgs.pwgen ];
        # todo maybe use writers here, with environment variables
        script = ''
          pwgen -s 48 1 > "$out/oidc_secret"
        '';
      };

    };
  };

}
