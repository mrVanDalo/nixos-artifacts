#!/usr/bin/env bash
# Unified check script for shared artifacts
# Uses the same interface as single-target check.sh

exec "$(dirname "$0")/check.sh"
