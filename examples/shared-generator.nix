{ pkgs, ... }:
{
  artifacts.store.nextcloud-oidc = {
    shared = true;
    files.secret = { };
    generator = pkgs.writers.writeBash "generate-nextcloud-oidc" ''
      ${pkgs.pwgen}/bin/pwgen -s 48 1 > "$out/secret"
    '';
  };
}
