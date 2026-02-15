# Example Usage

## Quick Start

1. Enter the development shell:
   ```bash
   nix develop
   ```

2. Run the artifacts TUI:
   ```bash
   artifacts
   ```

3. Generate an artifact (follow the prompts in the TUI)

4. Check the generated secrets:
   ```bash
   ls -la secrets/
   ```

## Example Output

After generating artifacts for a machine called "server-one" with an artifact
"ssh-key":

```
secrets/
└── machines/
    └── server-one/
        └── ssh-key/
            ├── id_ed25519
            └── id_ed25519.pub
```

For a shared artifact "ca-cert":

```
secrets/
└── shared/
    └── ca-cert/
        └── ca.crt
```

## Environment Variables

- `NIXOS_ARTIFACTS_PROJECT_ROOT` - Override the project root directory (default:
  current directory)

## Notes

- The `secrets/` directory is automatically ignored by git
- Each run regenerates all secrets (check.sh always returns 1)
- Secrets are stored in plain text - do not use in production
