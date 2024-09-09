#!/bin/bash

set -e

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

cd "$SCRIPT_DIR/.."

for dir in $(git ls-files | grep -P 'Cargo\.toml')
do
    pushd "$(dirname -- $dir)"
    echo "formatting $dir"
    cargo fmt
    popd
done
