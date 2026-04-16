{
  lib,
  ...
}:
with lib;
with types;
{
  options = {

    artifacts.default.backend = mkOption {
      type = str;
      description = ''
        Name of the default backend used to (de)serialize artifacts.

        Every artifact whose `artifacts.store.<name>.backend` is not set
        explicitly falls back to this value. The name must match a backend
        provided via `mkArtifactCli`.
      '';
    };

  };

}
