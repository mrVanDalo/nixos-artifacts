#!/usr/bin/env bash
# Verify $machines and $users during serialization for shared artifacts

# Verify $machines environment variable is set
if [ -z "$machines" ]; then
    echo "ERROR: \$machines environment variable is not set during shared serialize" >&2
    exit 1
fi

# Verify $users environment variable is set
if [ -z "$users" ]; then
    echo "ERROR: \$users environment variable is not set during shared serialize" >&2
    exit 1
fi

# Verify the machines file exists and starts with { (JSON object)
if [ ! -f "$machines" ]; then
    echo "ERROR: \$machines file does not exist during shared serialize: $machines" >&2
    exit 1
fi

first_char=$(head -c 1 "$machines" 2>/dev/null)
if [ "$first_char" != "{" ]; then
    echo "ERROR: \$machines does not appear to be JSON object during shared serialize:" >&2
    cat "$machines" >&2
    exit 1
fi

# Verify the users file exists and starts with { (JSON object)
if [ ! -f "$users" ]; then
    echo "ERROR: \$users file does not exist during shared serialize: $users" >&2
    exit 1
fi

first_char=$(head -c 1 "$users" 2>/dev/null)
if [ "$first_char" != "{" ]; then
    echo "ERROR: \$users does not appear to be JSON object during shared serialize:" >&2
    cat "$users" >&2
    exit 1
fi

# Output for snapshot verification
echo "# BEGIN SHARED SERIALIZE SNAPSHOT"
echo "# artifact=$artifact"
echo "# out=$out"
echo "# MACHINES FILE:"
cat "$machines"
echo "# USERS FILE:"
cat "$users"
echo "# END SHARED SERIALIZE SNAPSHOT"

# Copy artifacts to test output directory if set
if [ -n "$ARTIFACTS_TEST_OUTPUT_DIR" ]; then
    # For shared artifacts, we copy to all targets
    # Read machine names from machines.json using grep (portable)
    # machines.json is { "machine1": {...}, "machine2": {...} }
    for machine_name in $(grep -o '"[^"]*":' "$machines" 2>/dev/null | tr -d '":' | head -20); do
        target_dir="$ARTIFACTS_TEST_OUTPUT_DIR/machines/$machine_name/$artifact"
        mkdir -p "$target_dir"
        cp -r "$out"/* "$target_dir/" 2>/dev/null || true
    done
    
    # Read user names from users.json
    for user_name in $(grep -o '"[^"]*":' "$users" 2>/dev/null | tr -d '":' | head -20); do
        target_dir="$ARTIFACTS_TEST_OUTPUT_DIR/users/$user_name/$artifact"
        mkdir -p "$target_dir"
        cp -r "$out"/* "$target_dir/" 2>/dev/null || true
    done
fi