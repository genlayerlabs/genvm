#!/usr/bin/bash
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
source "/scripts/common.sh"

cd /opt/cpython

mkdir -p cross-build/wasm32-wasi
pushd cross-build/wasm32-wasi

# --with-openssl="$WASM32_WASI_ROOT/ssl" \
# OPENSSL_LIBS="$(pkg-config --libs-only-l openssl)" \

env \
    CC=/opt/wasi-sdk-24.0/bin/clang \
    CFLAGS="-O3 -g0 --sysroot=/opt/wasi-sdk-24.0/share/wasi-sysroot --target=wasm32-wasip1 -I$WASM32_WASI_ROOT/include -Wno-builtin-macro-redefined -D__TIME__='\"0:42:42\"' -D__DATE__='\"Jan/24/2024\"'" \
    LDFLAGS="-L$WASM32_WASI_ROOT/lib" \
    CONFIG_SITE="/opt/cpython/Tools/wasm/config.site-wasm32-wasi" \
    ../../configure \
        --prefix /out/py \
        --host=wasm32-wasi "--build=$(gcc -print-multiarch)" \
        --with-build-python=/opt/cpython/cross-build/build/python \
        --with-ensurepip=no --disable-ipv6 --disable-test-modules
make clean
cp /scripts-py/python-setup.local Modules/Setup.local
make -j
make install

/scripts-py/compile.sh /out/py/lib/python3.13

rm -rf /out/to-zip/ || true
mkdir -p /out/to-zip/
cp -r /out/py/lib/python3.13 /out/to-zip/
mv /out/to-zip/python3.13 /out/to-zip/py
cd /out/to-zip/py
find . -type f -not -name '*.py' -and -not -name '*.py' -and -not -name 'LICENSE*' -delete
rm -rf idlelib
rm -rf turtle
mkdir -p lib-dynload
touch lib-dynload/.keep
cp /out/py/bin/python3.wasm /out/cpython.raw.wasm
/opt/wabt-1.0.36/bin/wasm-strip /out/cpython.raw.wasm

chmod -R a+rw /out/to-zip /out/cpython.raw.wasm

cd /out/to-zip
zip -r ../cpython.zip *

:> /out/checksums
find /opt/wasm32-wasip1-root/ -type f | sort | xargs sha256sum >> /out/checksums
find /opt/cpython/cross-build/wasm32-wasi/Programs /opt/cpython/cross-build/wasm32-wasi/Python/ -type f -name '*.o' | sort | xargs sha256sum >> /out/checksums
