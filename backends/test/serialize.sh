#!/usr/bin/env bash
set -e

# Serialize secrets to filesystem
project_root="${NIXOS_ARTIFACTS_PROJECT_ROOT:-$(pwd)}"
secrets_dir="$project_root/secrets"

# Determine target based on context
if [ -n "$machine" ]; then
  target_dir="$secrets_dir/machines/$machine/$artifact"
elif [ -n "$user" ]; then
  target_dir="$secrets_dir/user/$user/$artifact"
else
  echo "Error: Neither \$machine nor \$user is set" >&2
  exit 1
fi

mkdir -p "$target_dir"

# Copy generated files
for file in "$out"/*; do
  if [ -f "$file" ]; then
    cp "$file" "$target_dir/"
    echo "Serialized $(basename "$file") to $target_dir"
  fi
done
