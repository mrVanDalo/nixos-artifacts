#!/usr/bin/env bash
# Verify $machines and $users for shared artifacts
# Note: $config is NOT available for shared artifacts - configs are in machines/users JSON

# Verify $machines environment variable is set
if [ -z "$machines" ]; then
    echo "ERROR: \$machines environment variable is not set for shared artifact" >&2
    exit 1
fi

# Verify $users environment variable is set
if [ -z "$users" ]; then
    echo "ERROR: \$users environment variable is not set for shared artifact" >&2
    exit 1
fi

# Verify the machines file exists and starts with { (JSON object)
if [ ! -f "$machines" ]; then
    echo "ERROR: \$machines file does not exist: $machines" >&2
    exit 1
fi

first_char=$(head -c 1 "$machines" 2>/dev/null)
if [ "$first_char" != "{" ]; then
    echo "ERROR: \$machines does not appear to be JSON object:" >&2
    cat "$machines" >&2
    exit 1
fi

# Verify the users file exists and starts with { (JSON object)
if [ ! -f "$users" ]; then
    echo "ERROR: \$users file does not exist: $users" >&2
    exit 1
fi

first_char=$(head -c 1 "$users" 2>/dev/null)
if [ "$first_char" != "{" ]; then
    echo "ERROR: \$users does not appear to be JSON object:" >&2
    cat "$users" >&2
    exit 1
fi

# Output for snapshot verification
echo "# BEGIN SHARED CHECK SNAPSHOT"
echo "# artifact=$artifact"
echo "# MACHINES FILE:"
cat "$machines"
echo "# USERS FILE:"
cat "$users"
echo "# END SHARED CHECK SNAPSHOT"

# Always request generation (for test purposes)
exit 1