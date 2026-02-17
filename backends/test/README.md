# File Backend

Simple file-based backend for development and testing. **Not secure** - stores
secrets in plain text.

## Directory Structure

Secrets are serialized to:

```
secrets/
├── machines/<machine>/<artifact>/<file>
├── user/<user>/<artifact>/<file>
└── shared/<artifact>/<file>
```

## Usage

The file backend is configured in `nix/devshells.nix` and automatically
available in `nix develop`.

When you run `artifacts`, secrets are serialized to the `secrets/` directory in
your project root.

## Scripts

- `check.sh` - Always regenerates (returns 1)
- `serialize.sh` - Copies files to machine/user directories
- `shared-serialize.sh` - Copies files to shared directory

## Security Warning

**Plain text storage only.** Use only for:

- Local development
- Testing
- CI/CD with proper access controls

Never use in production. The `secrets/` directory is gitignored.
