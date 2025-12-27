#!/usr/bin/env bash

# Print environment variables provided by artifacts-cli
echo "Environment variables:"
echo "out=$out"
echo "config=$config"
echo "machine=$machine"
echo "artifact=$artifact"
echo "username=$username"
echo "artifact_context=$artifact_context"

# If config points to a file, show its content for visibility
if [ -n "$config" ] && [ -f "$config" ]; then
    echo "--- Begin $config ---"
    cat "$config" | jq
    echo "--- End $config ---"
fi

echo

# Show contents of generated output files
for file in "$out"/*; do
    if [ -f "$file" ]; then
        echo "=== Content of $file ==="
        cat "$file"
        echo "========================="
    fi
done
