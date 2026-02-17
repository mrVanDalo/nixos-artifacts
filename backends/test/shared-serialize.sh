#!/usr/bin/env bash
set -e

# Serialize shared secrets to filesystem
project_root="${NIXOS_ARTIFACTS_PROJECT_ROOT:-$(pwd)}"
secrets_dir="$project_root/secrets"
target_dir="$secrets_dir/shared/$artifact"

mkdir -p "$target_dir"

# Copy generated files
for file in "$out"/*; do
  if [ -f "$file" ]; then
    cp "$file" "$target_dir/"
    echo "Serialized $(basename "$file") to $target_dir"
  fi
done
