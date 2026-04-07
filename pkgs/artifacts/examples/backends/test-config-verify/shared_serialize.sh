#!/usr/bin/env bash
# Unified serialize script for shared artifacts
# Uses the same interface as single-target serialize.sh

exec "$(dirname "$0")/serialize.sh"
