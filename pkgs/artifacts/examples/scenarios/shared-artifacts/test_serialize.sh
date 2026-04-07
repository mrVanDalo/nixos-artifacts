#!/usr/bin/env bash
# Test serialization - copy files to test output directory
set -e

if [ -n "$ARTIFACTS_TEST_OUTPUT_DIR" ]; then
    # Use jq to parse targets.json for target info
    context=$(jq -r '.context' "$targets")

    if [ "$context" = "shared" ]; then
        target_dir="$ARTIFACTS_TEST_OUTPUT_DIR/shared/$artifact"
    else
        # Single target - get the first (only) target
        target_name=$(jq -r '.targets[0].name' "$targets")
        target_type=$(jq -r '.targets[0].type' "$targets")

        if [ "$target_type" = "homemanager" ]; then
            target_dir="$ARTIFACTS_TEST_OUTPUT_DIR/users/$target_name/$artifact"
        else
            target_dir="$ARTIFACTS_TEST_OUTPUT_DIR/machines/$target_name/$artifact"
        fi
    fi
    mkdir -p "$target_dir"
    cp -r "$out"/* "$target_dir/"
fi
