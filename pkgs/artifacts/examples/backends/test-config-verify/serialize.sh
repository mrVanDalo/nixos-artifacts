#!/usr/bin/env bash
# Unified serialize script for all contexts (nixos, homemanager, shared)
# Verifies the unified environment variables are set correctly.

# Verify $artifact environment variable is set
if [ -z "$artifact" ]; then
    echo "ERROR: \$artifact is not set" >&2
    exit 1
fi

# Verify $artifact_context is set
if [ -z "$artifact_context" ]; then
    echo "ERROR: \$artifact_context is not set" >&2
    exit 1
fi

# Verify $targets is set and file exists
if [ -z "$targets" ]; then
    echo "ERROR: \$targets is not set" >&2
    exit 1
fi

if [ ! -f "$targets" ]; then
    echo "ERROR: \$targets file does not exist: $targets" >&2
    exit 1
fi

# Verify it contains JSON-like content (starts with {)
first_char=$(head -c 1 "$targets" 2>/dev/null)
if [ "$first_char" != "{" ]; then
    echo "ERROR: \$targets does not appear to be JSON (starts with '$first_char'):" >&2
    cat "$targets" >&2
    exit 1
fi

# Verify $out is set and directory exists
if [ -z "$out" ]; then
    echo "ERROR: \$out is not set" >&2
    exit 1
fi

if [ ! -d "$out" ]; then
    echo "ERROR: \$out directory does not exist: $out" >&2
    exit 1
fi

# Output for snapshot verification
echo "# BEGIN SERIALIZE SNAPSHOT"
echo "# artifact=$artifact"
echo "# artifact_context=$artifact_context"
echo "# TARGETS FILE:"
cat "$targets"
echo "# OUTPUT DIRECTORY:"
ls -1 "$out" 2>/dev/null | sort
echo "# END SERIALIZE SNAPSHOT"

# Copy artifacts to test output directory if set
if [ -n "$ARTIFACTS_TEST_OUTPUT_DIR" ]; then
    # Use jq to parse targets and determine storage location
    context=$(jq -r '.context' "$targets")

    if [ "$context" = "shared" ]; then
        target_dir="$ARTIFACTS_TEST_OUTPUT_DIR/shared/$artifact"
        mkdir -p "$target_dir"
        cp -r "$out"/* "$target_dir/" 2>/dev/null || true
    else
        # Single target - get the first (only) target
        target_name=$(jq -r '.targets[0].name' "$targets")
        target_type=$(jq -r '.targets[0].type' "$targets")

        if [ "$target_type" = "homemanager" ]; then
            target_dir="$ARTIFACTS_TEST_OUTPUT_DIR/users/$target_name/$artifact"
        else
            target_dir="$ARTIFACTS_TEST_OUTPUT_DIR/machines/$target_name/$artifact"
        fi
        mkdir -p "$target_dir"
        cp -r "$out"/* "$target_dir/" 2>/dev/null || true
    fi
fi

exit 0
