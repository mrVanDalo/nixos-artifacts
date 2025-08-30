{ pkgs, ... }:
{

  artifacts.store.test = {
    files.secret = { };
    prompts.test.description = "test input";
    generator = pkgs.writers.writeBash "test" ''
      cat $prompt/test > $out/secret
    '';
  };

}
