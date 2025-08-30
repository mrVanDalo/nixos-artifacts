{
  pkgs ? import <nixpkgs> { },
}:

pkgs.rustPlatform.buildRustPackage rec {
  pname = "artifacts-tui";
  version = "0.1.0";

  src = ./.;

  # Use the lock file shipped with the repository
  cargoLock.lockFile = ./Cargo.lock;

  doCheck = false;

  # Set to lib.fakeSha256 initially. Build once to get the real cargoHash from the error message,
  # then replace this value for reproducible builds.
  cargoHash = pkgs.lib.fakeSha256;

  # Ensure a modern Rust toolchain compatible with edition = "2024"
  # (override if your nixpkgs already provides a new enough rustc).
  #
  # Example of pinning (uncomment and adjust if needed):
  # rustVersion = "1.79.0";

  # No extra nativeBuildInputs required for this project.

  meta = with pkgs.lib; {
    description = "TUI for managing NixOS artifacts";
    homepage = "https://github.com/"; # update if/when a homepage exists
    license = licenses.mit; # adjust if different
    maintainers = [ ];
    platforms = platforms.unix;
    mainProgram = "artifacts-cli";
  };
}
