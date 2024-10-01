#!/usr/bin/env bash

set -ex

export PATH="$HOME/.cargo/bin:$PATH"

cd /opt/genvm/runners/cpython-and-ext/extension/
cargo build --target wasm32-wasip1 --profile release
cp target/wasm32-wasip1/release/libgenvm_cpython_ext.a /out
cd /out
sha256sum libgenvm_cpython_ext.a > sum
chmod -R a+rw /out/
