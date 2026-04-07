#!/usr/bin/env bash
# Skip generation for machine-one/artifact-two, require it for others
# Use the unified targets.json to get machine name

if [ -z "$targets" ] || [ -z "$artifact" ]; then
    exit 1
fi

# Get machine name from targets.json
machine=$(jq -r '.targets[0].name' "$targets")

if [ -z "$machine" ]; then
    exit 1
fi

if [ "$machine" = "machine-one" ] && [ "$artifact" = "artifact-two" ]; then
    exit 0
fi

exit 1
