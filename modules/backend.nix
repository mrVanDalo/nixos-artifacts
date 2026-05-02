{
  lib,
  ...
}:
with lib;
with types;
{

  # Backend selection options.
  #
  # `artifacts.default.backend` declares which backend serializes an artifact
  # when `artifacts.store.<name>.backend` is not set explicitly. The actual
  # serialization scripts and per-target environment (including the unified
  # `$targets` JSON, `$artifact`, `$artifact_context`, `$out`, `$inputs`,
  # `$LOG_LEVEL`) are owned by the backend package — see
  # `docs/modules/ROOT/pages/backend-scripts-reference.adoc`.

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
