#/bin/bash
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
source "$SCRIPT_DIR/common.sh"

cd /opt/cpython

mkdir -p cross-build/wasm32-wasi
pushd cross-build/wasm32-wasi

# --with-openssl="$WASM32_WASI_ROOT/ssl" \
# OPENSSL_LIBS="$(pkg-config --libs-only-l openssl)" \

env \
    CC=/opt/wasi-sdk-24.0/bin/clang \
    CFLAGS="-O3 -g0 --sysroot=/opt/wasi-sdk-24.0/share/wasi-sysroot -I$WASM32_WASI_ROOT/include -Wno-builtin-macro-redefined -D__TIME__='\"0:42:42\"' -D__DATE__='\"Jan/24/2024\"'" \
    LDFLAGS="-L$WASM32_WASI_ROOT/lib" \
    CONFIG_SITE="/opt/cpython/Tools/wasm/config.site-wasm32-wasi" \
    ../../configure \
        --prefix /out/py \
        --host=wasm32-wasi "--build=$(gcc -print-multiarch)" \
        --with-build-python=/opt/cpython/cross-build/build/python \
        --with-ensurepip=no --disable-ipv6 --disable-test-modules
make clean
make -j

#./configure --host=wasm32-wasi "--build=$(gcc -print-multiarch)" --with-build-python=/usr/bin/python3
