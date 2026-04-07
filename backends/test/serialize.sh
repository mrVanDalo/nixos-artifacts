#!/usr/bin/env bash
set -e

# Serialize secrets to filesystem
# Uses unified environment: $artifact, $artifact_context, $targets, $out
project_root="${NIXOS_ARTIFACTS_PROJECT_ROOT:-$(pwd)}"
secrets_dir="$project_root/secrets"

# Parse targets.json to determine storage location
context=$(jq -r '.context' "$targets")

if [ "$context" = "shared" ]; then
  echo "serialize to shared"
  target_dir="$secrets_dir/shared/$artifact"
else
  # Single target - get the first (only) target
  target_name=$(jq -r '.targets[0].name' "$targets")
  target_type=$(jq -r '.targets[0].type' "$targets")

  if [ "$target_type" = "homemanager" ]; then
    echo "serialize to user"
    target_dir="$secrets_dir/user/$target_name/$artifact"
  else
    echo "serialize to machine"
    target_dir="$secrets_dir/machines/$target_name/$artifact"
  fi
fi

mkdir -p "$target_dir"

# Copy generated files
for file in "$out"/*; do
  if [ -f "$file" ]; then
    cp "$file" "$target_dir/"
    echo "Serialized $(basename "$file") to $target_dir"
  fi
done
