#!/bin/bash

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

cd "$SCRIPT_DIR"

cargo build --release --features sdk-debug
FROM="$(readlink -f target/wasm32-wasi/release/genvm-python.wasm)"
TO="$(readlink -f target/wasm32-wasi/release/genvm-python.nof.wasm)"

cd ../tools/softfloat-lib/patch-floats
cargo run "$FROM" "$TO"
