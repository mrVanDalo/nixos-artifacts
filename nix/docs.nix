{ inputs, ... }:
{
  perSystem =
    { pkgs, self', ... }:
    let

      antoraCommand = pkgs.writeShellApplication {
        name = "antora-command";
        runtimeInputs = [ pkgs.antora ];
        text = ''
          set -euo pipefail
          export ANTORA_CACHE_DIR="$PWD/.cache"
          echo "Building documentation..."
          cd docs
          antora \
            --stacktrace \
            --to-dir ../build/site \
            antora-playbook.yml
          echo
          echo "✅ Documentation built successfully!"
          echo "Site generated in: build/site"
          cd ..
        '';
      };

      serveDocsScript = pkgs.writeShellApplication {
        name = "serve-docs";
        runtimeInputs = [ pkgs.python3 ];
        text = ''
          set -euo pipefail

          ${antoraCommand}/bin/antora-command

          echo "Starting local server at http://localhost:8000"
          echo "Press Ctrl+C to stop"
          cd build/site
          python3 -m http.server 8000
        '';
      };

      watchDocsScript = pkgs.writeShellApplication {
        name = "watch-docs";
        runtimeInputs = [ pkgs.watchexec ];
        text = ''
          set -euo pipefail
          echo "👀 Watching docs/ folder for changes..."
          echo "Press Ctrl+C to stop"
          watchexec \
            --watch docs \
            --exts adoc,yml,yaml \
            ${antoraCommand}/bin/antora-command
        '';
      };

      rustDocScript = pkgs.writeShellApplication {
        name = "rust-doc";
        runtimeInputs = [ pkgs.rustup ];
        text = ''
          set -euo pipefail
          pushd pkgs/artifacts
          echo "Building Rust API documentation..."

          export RUSTUP_HOME="$PWD/.rustup"
          export CARGO_HOME="$PWD/.cargo"
          export RUSTUP_TOOLCHAIN=1.87.0

          rustup default 1.87.0 --quiet 2>/dev/null || true
          rustup component add rust-docs --quiet 2>/dev/null || true

          cargo doc --lib --no-deps --document-private-items 2>&1 || true

          echo ""
          echo "✅ Rust API documentation built!"
          echo "Open: $PWD/target/doc/artifacts/index.html"
          popd
        '';
      };

      serveRustDocScript = pkgs.writeShellApplication {
        name = "serve-rust-doc";
        runtimeInputs = [
          pkgs.rustup
          pkgs.python3
        ];
        text = ''
          set -euo pipefail
          pushd pkgs/artifacts
          echo "Building Rust API documentation..."

          export RUSTUP_HOME="$PWD/.rustup"
          export CARGO_HOME="$PWD/.cargo"
          export RUSTUP_TOOLCHAIN=1.87.0

          rustup default 1.87.0 --quiet 2>/dev/null || true
          rustup component add rust-docs --quiet 2>/dev/null || true

          cargo doc --lib --no-deps --document-private-items 2>&1 || true

          echo ""
          echo "✅ Rust API documentation built!"
          echo "Starting local server at http://localhost:8000"
          echo "Press Ctrl+C to stop"
          cd target/doc
          python3 -m http.server 8000
          popd
        '';
      };
    in
    {
      apps = {
        build-docs = {
          type = "app";
          program = "${antoraCommand}/bin/antora-command";
        };

        serve-docs = {
          type = "app";
          program = "${serveDocsScript}/bin/serve-docs";
        };

        watch-docs = {
          type = "app";
          program = "${watchDocsScript}/bin/watch-docs";
        };

        rust-doc = {
          type = "app";
          program = "${rustDocScript}/bin/rust-doc";
        };

        serve-rust-doc = {
          type = "app";
          program = "${serveRustDocScript}/bin/serve-rust-doc";
        };
      };
    };
}
