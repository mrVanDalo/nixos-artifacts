#!/usr/bin/env bash
# Test shared serialization - output env vars for debugging
echo "artifact=$artifact"
echo "out=$out"
echo "machines content:"
cat "$machines"
echo "users content:"
cat "$users"
