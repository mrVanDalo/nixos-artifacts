#!/usr/bin/env bash
set -euo pipefail

# Change to the repository root (directory of this script)
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Build the project
echo "[test-generate] Building artifacts-tui..."
cargo build -q

# Run the generate command with the provided example configs
echo "[test-generate] Running generate with example configs..."
cmd=(cargo run --quiet -- generate src/examples/backend.toml src/examples/make.json)
echo "> ${cmd[*]}"
"${cmd[@]}"

echo "[test-generate] Done."
