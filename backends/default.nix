{ lib, pkgs }:

backendName:
{ nixos_check_serialization
, nixos_serialize
, home_check_serialization
, home_serialize
, shared_check_serialization ? null
, shared_serialize ? null
, settings ? { }
, capabilities ? { shared = false; serializes = true; }
, enabled ? true
}:

let
  toml = pkgs.formats.toml { };

  backendToml = {
    ${backendName} = {
      nixos_check_serialization = "./nixos_check_serialization.sh";
      nixos_serialize = "./nixos_serialize.sh";
      home_check_serialization = "./home_check_serialization.sh";
      home_serialize = "./home_serialize.sh";
    } // lib.optionalAttrs (shared_check_serialization != null) {
      shared_check_serialization = "./shared_check_serialization.sh";
    } // lib.optionalAttrs (shared_serialize != null) {
      shared_serialize = "./shared_serialize.sh";
    } // lib.optionalAttrs (settings != { }) {
      inherit settings;
    } // {
      inherit capabilities enabled;
    };
  };

  backendConfigFile = toml.generate "backend.toml" backendToml;
in

pkgs.runCommand "${backendName}-backend" { } ''
  mkdir -p $out

  cp ${backendConfigFile} $out/backend.toml

  cp ${nixos_check_serialization} $out/nixos_check_serialization.sh
  chmod +x $out/nixos_check_serialization.sh

  cp ${nixos_serialize} $out/nixos_serialize.sh
  chmod +x $out/nixos_serialize.sh

  cp ${home_check_serialization} $out/home_check_serialization.sh
  chmod +x $out/home_check_serialization.sh

  cp ${home_serialize} $out/home_serialize.sh
  chmod +x $out/home_serialize.sh

  ${lib.optionalString (shared_check_serialization != null) ''
    cp ${shared_check_serialization} $out/shared_check_serialization.sh
    chmod +x $out/shared_check_serialization.sh
  ''}

  ${lib.optionalString (shared_serialize != null) ''
    cp ${shared_serialize} $out/shared_serialize.sh
    chmod +x $out/shared_serialize.sh
  ''}
''