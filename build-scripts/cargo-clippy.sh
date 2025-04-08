#!/usr/bin/env bash

set -e

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

cd "$SCRIPT_DIR/.."

if ! cargo clippy --version
then
    echo "WARNING: cargo clippy not installed"
    exit 0
fi

for dir in $(git ls-files | grep -P 'Cargo\.toml')
do
    pushd "$(dirname -- $dir)" 2> /dev/null > /dev/null
    echo "clippy in $dir"
    cargo clippy --target-dir "$SCRIPT_DIR/../build/generated/rust-target" -- -A clippy::upper_case_acronyms -Dwarnings
    popd 2> /dev/null > /dev/null
done
