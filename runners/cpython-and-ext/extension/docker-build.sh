#!/usr/bin/env bash

set -ex

export PATH="$HOME/.cargo/bin:$PATH"

cd /opt/genvm/runners/cpython-and-ext/extension/
cargo build --target wasm32-wasip1 --profile release

ls target/wasm32-wasip1/release/
ls -Ra target/wasm32-wasip1/release/deps

cp target/wasm32-wasip1/release/libgenvm_cpython_ext.a /out/libgenvm_cpython_ext.a

#/opt/genvm/tools/downloaded/wasi-sdk-24/bin/clang \
#    -shared \
#    -Wl,--export-all \
#    -Wl,-no-gc-sections \
#    -Wl,--export-dynamic \
#    -o /out/_wasi.raw.so \
#    /out/libgenvm_cpython_ext.a

cd /out
sha256sum libgenvm_cpython_ext.a > sum
chmod -R a+rw /out/
