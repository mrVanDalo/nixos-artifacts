{ self, ... }:
{
  perSystem =
    { system, ... }:
    let
      testBackend = self.lib.mkBackend {
        inherit system;
        name = "test";
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
    in
    {
      packages.example-backend = testBackend;
    };
}
