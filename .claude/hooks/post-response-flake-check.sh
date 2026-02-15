#!/usr/bin/env bash
# Run 'nix flake check' after file changes

set -euo pipefail

# Exit if no files changed
if git diff --quiet HEAD 2>/dev/null; then
  exit 0
fi

echo "🔍 Running nix flake check..."

# Run check, show output on failure
if ! nix flake check 2>&1; then
  echo "❌ nix flake check failed"
  exit 1
fi

echo "✅ nix flake check passed"
