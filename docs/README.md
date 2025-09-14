# NixOS Artifacts Docs (Antora)

This is a minimal Antora setup primarily to demonstrate the dev shell and file layout for building docs that can be published via GitHub Pages.

## Structure

- `docs/antora.yml` — Component descriptor
- `docs/antora-playbook.yml` — Antora playbook (build config)
- `docs/modules/ROOT/pages/index.adoc` — Landing page
- `docs/modules/ROOT/nav.adoc` — Simple navigation
- `docs/public/` — Generated static site output (ignored)

## Build locally with Nix

Run the flake app to build the site:

```sh
nix run .#build-docs
```

Open the generated site at `docs/public/index.html` in your browser.

## GitHub Pages

You can publish the `docs/public` folder with GitHub Pages (e.g., using a workflow that runs `nix run .#build-docs` and uploads `docs/public`).
