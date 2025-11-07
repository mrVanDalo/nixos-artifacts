#!/usr/bin/env bash

# Print environment variables provided by artifacts-cli
echo "Environment variables:"
echo "inputs=$inputs"
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

if [ -z "$machine" ] || [ -z "$artifact" ]; then
    exit 1
fi

if [ "$machine" = "machine-one" ] && [ "$artifact" = "artifact-two" ]; then
    exit 0
fi

exit 1