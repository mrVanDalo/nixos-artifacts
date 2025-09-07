#!/usr/bin/env bash

echo "machine=$machine"
echo "artifact=$artifact"

cat $prompts/secret1 > $out/very-simple-secrets
cat $prompts/secret2 > $out/simple-secrets
