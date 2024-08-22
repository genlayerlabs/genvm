#!/bin/bash

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

cd "$SCRIPT_DIR"
export RUSTFLAGS=-Awarnings
HOST="$(rustc --version --verbose | grep 'host' | sed -e 's/host: //')"
cargo run --bin genvm-python-build-debug-sdk --features sdk-debug --target "$HOST"
