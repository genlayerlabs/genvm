#!/usr/bin/env bash
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
source "$SCRIPT_DIR/common.sh"

cd /opt/zlib-1.3.1

C_AND_CXX_FLAGS="-O3 --sysroot=/opt/host-root/share/wasi-sysroot --target=wasm32-wasi -fPIC "
CC=/opt/host-root/bin/clang CFLAGS="$C_AND_CXX_FLAGS" ./configure \
    "--prefix=$WASM32_WASI_ROOT"

make -j
make install
make clean || true
