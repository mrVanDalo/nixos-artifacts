#!/usr/bin/env bash
# Shared serialize - delegates to unified serialize.sh
# Since the interface is now unified, shared uses the same script as single
exec "$(dirname "$0")/serialize.sh"
