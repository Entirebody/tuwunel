#!/usr/bin/env bash

for f in $(rg --no-config --engine=default -l --sort=path '\bpub\([^)]+\)' src); do
    echo "Fixing '$f'"
    sed -i -E 's/\<pub\([^)]+\)/pub/g' "$f"
done
