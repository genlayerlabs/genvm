#!/usr/bin/env bash
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
source "/scripts/common.sh"

cd /opt/cpython

mkdir -p cross-build/wasm32-wasi
pushd cross-build/wasm32-wasi

# --with-openssl="$WASM32_WASI_ROOT/ssl" \
# OPENSSL_LIBS="$(pkg-config --libs-only-l openssl)" \

env \
    CC=/opt/host-root/bin/clang \
    CFLAGS="-O3 -g --sysroot=/opt/host-root/share/wasi-sysroot --target=wasm32-wasip1 -I$WASM32_WASI_ROOT/include $DETERMINISTIC_C_FLAGS" \
    CONFIG_SITE="/opt/cpython/Tools/wasm/config.site-wasm32-wasi" \
    LDFLAGS="-L$WASM32_WASI_ROOT/lib" \
    ../../configure \
        --prefix /opt/wasm32-wasip1-root/ \
        --config-cache \
        --host=wasm32-wasi "--build=$(gcc -print-multiarch)" \
        --with-build-python=/opt/cpython/cross-build/build/python \
        --with-ensurepip=no --disable-ipv6 --disable-test-modules
make clean
make -j inclinstall libainstall
make clean || true
