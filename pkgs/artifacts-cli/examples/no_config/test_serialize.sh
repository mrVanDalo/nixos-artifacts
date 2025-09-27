#!/usr/bin/env bash

echo '$config: '
cat $config
echo

for file in "$out"/*; do
    if [ -f "$file" ]; then
        echo "=== Content of $file ==="
        cat "$file"
        echo "========================="
    fi
done
