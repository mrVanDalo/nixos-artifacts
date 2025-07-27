{
  pkgs,
  config,
  lib,
  ...
}:
{
  options = {

    artifacts.example.shared.enable = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = "Enable shared artifacts example functionality";
    };

  };

  config = lib.mkIf config.artifacts.example.shared.enable {
    artifacts.store.nextcloud-odic = {
      shared = true;

      files.secret = { };
      generator = pkgs.writers.writeBash "generate-nextcloud-oidc" ''
        cat >"$out/env" <<EOF
        ${pkgs.pwgen}/bin/pwgen -s 48 1 > "$out/secret"
        EOF
      '';
    };
  };
}
