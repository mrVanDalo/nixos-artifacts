{ inputs, ... }:
{
  perSystem =
    { pkgs, ... }:
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
      };
    };
}
