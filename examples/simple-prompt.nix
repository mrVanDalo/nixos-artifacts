{ pkgs, ... }:
{

  artifacts.store.test = {
    files.secret = { };
    prompts.test.description = "test input";
    generator = pkgs.writers.writeBash "test" ''
      cat $prompts/test > $out/secret
    '';
  };

}
