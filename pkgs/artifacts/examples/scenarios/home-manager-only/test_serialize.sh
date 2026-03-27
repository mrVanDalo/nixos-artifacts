#!/usr/bin/env bash
# Test serialization - copy files to test output directory
set -e

if [ -n "$ARTIFACTS_TEST_OUTPUT_DIR" ]; then
    # Create target directory based on context
    if [ "$artifact_context" = "homemanager" ]; then
        target_dir="$ARTIFACTS_TEST_OUTPUT_DIR/users/$username/$artifact"
    else
        target_dir="$ARTIFACTS_TEST_OUTPUT_DIR/machines/$machine/$artifact"
    fi
    mkdir -p "$target_dir"
    cp -r "$out"/* "$target_dir/"
fi