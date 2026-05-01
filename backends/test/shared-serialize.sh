#!/usr/bin/env bash
# Shared serialize - delegates to unified serialize.sh
# Since the interface is now unified, shared uses the same script as single

# Debug: dump every env var the artifacts CLI injected so we can verify what's
# being passed to backend serialize operations.
echo "=== test backend shared-serialize.sh: injected environment ==="
env | sort
echo "=============================================================="

exec "$(dirname "$0")/serialize.sh"
