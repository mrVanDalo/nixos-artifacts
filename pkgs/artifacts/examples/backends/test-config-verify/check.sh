#!/usr/bin/env bash
# Verify $config is set, file exists, and contains valid JSON
# Used for NixOS and Home Manager targets (not shared artifacts)

# Verify $config environment variable is set
if [ -z "$config" ]; then
    echo "ERROR: \$config environment variable is not set" >&2
    exit 1
fi

# Verify the file exists
if [ ! -f "$config" ]; then
    echo "ERROR: \$config file does not exist: $config" >&2
    exit 1
fi

# Verify it contains JSON-like content (starts with { or [)
first_char=$(head -c 1 "$config" 2>/dev/null)
if [ "$first_char" != "{" ] && [ "$first_char" != "[" ]; then
    echo "ERROR: \$config does not appear to be JSON (starts with '$first_char'):" >&2
    cat "$config" >&2
    exit 1
fi

# Output config content for snapshot verification
echo "# BEGIN CONFIG SNAPSHOT"
echo "# config=$config"
echo "# target_type=nixos_or_home"
echo "# machine=${machine:-}"
echo "# username=${username:-}"
echo "# artifact=$artifact"
echo "# artifact_context=$artifact_context"
echo "# FILE CONTENT:"
cat "$config"
echo "# END CONFIG SNAPSHOT"

# Always request generation (for test purposes)
exit 1