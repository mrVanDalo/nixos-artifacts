#!/usr/bin/env bash
set -euo pipefail

# Change to the repository root (directory of this script)
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Run clippy with warnings as errors via cargo alias
echo "[test-lint] Running cargo lint (clippy)..."
cmd=(cargo lint)
echo "> ${cmd[*]}"
"${cmd[@]}"

echo "[test-lint] Done."
