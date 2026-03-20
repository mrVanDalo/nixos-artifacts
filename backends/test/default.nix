{ self, ... }:
{
  perSystem =
    { system, ... }:
    let
      testBackend = self.lib.mkBackend {
        inherit system;
        name = "test";
        nixos_check = ./check.sh;
        nixos_serialize = ./serialize.sh;
        home_check = ./check.sh;
        home_serialize = ./serialize.sh;
        shared_check = ./check.sh;
        shared_serialize = ./shared-serialize.sh;
      };
    in
    {
      packages.example-backend = testBackend;
    };
}
