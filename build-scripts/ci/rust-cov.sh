#!/usr/bin/env bash

set -ex

if ! [ -d build/genvm-testdata-out/coverage ] || [ -z "$(ls -A build/genvm-testdata-out/coverage || true)" ]
then
    echo "no coverage generated"
    exit 1
fi

TOOLCHAIN=$(rustup show active-toolchain | grep -Po '^[a-zA-Z_\-0-9]+')
HOST=$(rustc --version --verbose | grep -Po 'host: .*' | sed -e 's/host: //g')

"$HOME/.rustup/toolchains/$TOOLCHAIN/lib/rustlib/$HOST/bin/llvm-profdata" \
    merge -sparse \
    build/genvm-testdata-out/coverage/*.profraw \
    -o build/genvm-testdata-out/coverage/result.profdata

"$HOME/.rustup/toolchains/$TOOLCHAIN/lib/rustlib/$HOST/bin/llvm-cov" \
    report \
    -instr-profile=build/genvm-testdata-out/coverage/result.profdata \
    --object build/out/bin/genvm --object build/out/lib/genvm-modules/* \
    --ignore-filename-regex='/\.cargo/registry|/third-party/|generated/rust-target|/rustc/'
