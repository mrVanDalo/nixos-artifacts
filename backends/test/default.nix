{ self' }:

self'.packages.artifacts.override {
  backends = {
    test = {
      nixos_check_serialization = ./check.sh;
      nixos_serialize = ./serialize.sh;
      home_check_serialization = ./check.sh;
      home_serialize = ./serialize.sh;
      shared_check_serialization = ./check.sh;
      shared_serialize = ./shared-serialize.sh;
      capabilities = {
        shared = true;
        serializes = true;
      };
    };
  };
}
