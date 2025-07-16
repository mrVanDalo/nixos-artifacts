{

  artifacts.store.gitea-runner = {
    # todo make this a oneliner
    prompt.token = "please enter your gitea-runner token";
    file.token = { };
    generator = {
      runtimeInputs = [ pkgs.coreutils ];
      script = ''
        cat $prompts/token > $out/token
      '';
    };
  };

  services.gitea-actions-runner = {
    instances."artifacts-example" = {
      enable = true;
      tokenFile = config.artifacts.store.gitea-runner.file."token".path;
    };
  };

}
