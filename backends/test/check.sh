#!/usr/bin/env bash
# Always regenerate (for development)
# Uses unified environment: $artifact, $artifact_context, $targets, $inputs
for input_file in "$inputs"/*; do
  [ -e "$input_file" ] || continue
  echo "$(basename "$input_file") does not exist yet"
done
exit 1
