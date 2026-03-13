{ ... }:
{
  perSystem =
    { self', ... }:
    {
      packages.rust-docs = self'.packages.artifacts-bin.overrideAttrs (old: {
        name = "artifacts-rust-docs";
        buildPhase = ''
          echo "Building Rust API documentation..."
          cargo doc --lib --no-deps --document-private-items 2>&1 || {
            echo "Warning: Documentation build had errors but continuing"
          }
        '';
        installPhase = ''
          mkdir -p $out/share/doc/artifacts-rust
          cp -r target/doc/* $out/share/doc/artifacts-rust/

          cat > $out/share/doc/artifacts-rust/index.html <<'HTML'
          <!DOCTYPE html>
          <html>
          <head>
            <meta charset="utf-8">
            <meta http-equiv="refresh" content="0; URL=artifacts/index.html">
            <title>Artifacts API Documentation</title>
          </head>
          <body>
            <p>Redirecting to <a href="artifacts/index.html">artifacts/index.html</a>...</p>
          </body>
          </html>
          HTML

          echo "Documentation installed to $out/share/doc/artifacts-rust/"
        '';
        checkPhase = "true";
      });
    };
}
