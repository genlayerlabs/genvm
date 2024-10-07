#!/usr/bin/env bash

set -e

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

cd "$SCRIPT_DIR/.."

if ! cargo fmt --version
then
    echo "WARNING: cargo fmt didn't exit successfully"
    exit 0
fi

for dir in $(git ls-files | grep -P 'Cargo\.toml')
do
    pushd "$(dirname -- $dir)" 2> /dev/null > /dev/null
    echo "formatting $dir"
    cargo fmt
    popd 2> /dev/null > /dev/null
done
