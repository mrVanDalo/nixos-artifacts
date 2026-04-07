#!/usr/bin/env bash
# Unified check script for all contexts (nixos, homemanager, shared)
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

# Verify $inputs is set and directory exists
if [ -z "$inputs" ]; then
    echo "ERROR: \$inputs is not set" >&2
    exit 1
fi

if [ ! -d "$inputs" ]; then
    echo "ERROR: \$inputs directory does not exist: $inputs" >&2
    exit 1
fi

# Output for snapshot verification
echo "# BEGIN CHECK SNAPSHOT"
echo "# artifact=$artifact"
echo "# artifact_context=$artifact_context"
echo "# TARGETS FILE:"
cat "$targets"
echo "# INPUTS DIRECTORY:"
ls -1 "$inputs" 2>/dev/null | sort
echo "# END CHECK SNAPSHOT"

# Always request generation (for test purposes)
exit 1
