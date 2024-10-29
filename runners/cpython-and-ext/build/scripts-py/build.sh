#!/usr/bin/env bash
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
source "/scripts/common.sh"

:> /out/checksums

cd /opt/cpython
mkdir -p cross-build/wasm32-wasi
pushd cross-build/wasm32-wasi

/scripts-py/modules/numpy/build.sh

mkdir -p /out/py/lib/python3.13

/opt/host-root/bin/clang -O3 -c /scripts-py/dl.c -o $WASM32_WASI_ROOT/lib/fake_dl.o

# -fPIC -fvisibility=default

env \
    CC=/opt/host-root/bin/clang \
    CFLAGS="-O3 -g --sysroot=/opt/host-root/share/wasi-sysroot --target=wasm32-wasip1 -I$WASM32_WASI_ROOT/include $DETERMINISTIC_C_FLAGS" \
    LDFLAGS="-L$WASM32_WASI_ROOT/lib $WASM32_WASI_ROOT/lib/fake_dl.o" \
    CONFIG_SITE="/opt/cpython/Tools/wasm/config.site-wasm32-wasi" \
    ../../configure \
        --prefix /out/py \
        --host=wasm32-wasi "--build=$(gcc -print-multiarch)" \
        --with-build-python=/opt/cpython/cross-build/build/python \
        --with-ensurepip=no --disable-ipv6 --disable-test-modules

cp /scripts-py/modules/cxxabi-stub.c ../../Modules/
cp /scripts-py/python-setup.local Modules/Setup.local
make clean
make -j
make install

cp -r /opt/np-built/lib/python3.13/site-packages/numpy /out/py/lib/python3.13
/scripts-py/compile.sh /out/py/lib/python3.13

rm -rf /out/to-zip/ || true
mkdir -p /out/to-zip/
cp -r /out/py/lib/python3.13 /out/to-zip/
mv /out/to-zip/python3.13 /out/to-zip/py
cd /out/to-zip/py
find . -type f -regextype posix-egrep -and -not -regex '.*\.(py|pyi|pxd|pyf|pyx)$' -and -not -name 'LICENSE*' -delete
find . -empty -type d -delete
rm -rf idlelib
rm -rf turtle
cp /out/py/bin/python3.wasm /out/cpython.raw.wasm
# /opt/wabt-1.0.36/bin/wasm-strip /out/cpython.raw.wasm

chmod -R a+rw /out/

cd /out/to-zip
zip -r ../cpython.zip *

find /opt/wasm32-wasip1-root/ -type f | sort | xargs sha256sum >> /out/checksums
find /opt/cpython/cross-build/wasm32-wasi/Programs /opt/cpython/cross-build/wasm32-wasi/Python/ -type f -name '*.o' | sort | xargs sha256sum >> /out/checksums

cat /out/checksums
