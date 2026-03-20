#!/usr/bin/env bash
# Verify $config during serialization for NixOS/Home targets

# Verify $config environment variable is set
if [ -z "$config" ]; then
    echo "ERROR: \$config environment variable is not set during serialize" >&2
    exit 1
fi

# Verify the file exists
if [ ! -f "$config" ]; then
    echo "ERROR: \$config file does not exist during serialize: $config" >&2
    exit 1
fi

# Verify it contains JSON-like content (starts with { or [)
first_char=$(head -c 1 "$config" 2>/dev/null)
if [ "$first_char" != "{" ] && [ "$first_char" != "[" ]; then
    echo "ERROR: \$config does not appear to be JSON during serialize (starts with '$first_char'):" >&2
    cat "$config" >&2
    exit 1
fi

# Output config content for snapshot verification
echo "# BEGIN SERIALIZE CONFIG SNAPSHOT"
echo "# config=$config"
echo "# target_type=nixos_or_home"
echo "# machine=${machine:-}"
echo "# username=${username:-}"
echo "# artifact=$artifact"
echo "# artifact_context=$artifact_context"
echo "# FILE CONTENT:"
cat "$config"
echo "# END SERIALIZE CONFIG SNAPSHOT"

# Copy artifacts to test output directory if set
if [ -n "$ARTIFACTS_TEST_OUTPUT_DIR" ]; then
    if [ "$artifact_context" = "homemanager" ]; then
        target_dir="$ARTIFACTS_TEST_OUTPUT_DIR/users/$username/$artifact"
    else
        target_dir="$ARTIFACTS_TEST_OUTPUT_DIR/machines/$machine/$artifact"
    fi
    mkdir -p "$target_dir"
    cp -r "$out"/* "$target_dir/"
fi