#!/usr/bin/env bash

# this actually is right
echo "test" > $out/very-simple-secrets
echo "test" > $out/simple-secrets

# this file should not exist
touch $out/should_not_be_there
