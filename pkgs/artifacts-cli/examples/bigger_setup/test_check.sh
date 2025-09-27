#!/usr/bin/env bash

if [ -z "$machine" ] || [ -z "$artifact" ]; then
    exit 1
fi

if [ "$machine" = "machine-one" ] && [ "$artifact" = "artifact-two" ]; then
    exit 0
fi

exit 1