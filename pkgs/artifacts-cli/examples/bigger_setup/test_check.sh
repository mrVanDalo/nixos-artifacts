#!/usr/bin/env bash
# Skip generation for machine-one/artifact-two, require it for others
if [ -z "$machine" ] || [ -z "$artifact" ]; then
    exit 1
fi

if [ "$machine" = "machine-one" ] && [ "$artifact" = "artifact-two" ]; then
    exit 0
fi

exit 1
