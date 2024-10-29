#!/usr/bin/env bash
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
source "$SCRIPT_DIR/common.sh"

cd /opt/libffi-3.4.6

env \
    CC=/opt/host-root/bin/clang \
    CFLAGS="--sysroot=/opt/host-root/share/wasi-sysroot/ -fPIC" \
    ./configure \
        --prefix=/opt/wasm32-wasip1-root/ \
        --host=wasm32-wasi

make install-pkgconfigDATA
make install-info
make install-data

ls /opt/wasm32-wasip1-root/include

AR_SCRIPT="CREATE /opt/wasm32-wasip1-root/lib/libffi.a"

for i in /scripts/stub_ffi.c /opt/libffi-3.4.6/src/closures.c /opt/libffi-3.4.6/src/prep_cif.c /opt/libffi-3.4.6/src/tramp.c /opt/libffi-3.4.6/src/debug.c /opt/libffi-3.4.6/src/raw_api.c /opt/libffi-3.4.6/src/types.c
do
    FNAME="$(basename "$i")"
    /opt/host-root/bin/clang \
        -o "/tmp/$FNAME.o" \
        -fPIC \
        -I/opt/wasm32-wasip1-root/include/ \
        -I/opt/libffi-3.4.6/include -I/opt/libffi-3.4.6/wasm32-unknown-wasi/ \
        -c "$i"
    AR_SCRIPT="$AR_SCRIPT"$'\n'"ADDMOD /tmp/$FNAME.o"
done

AR_SCRIPT="$AR_SCRIPT"$'\n'"SAVE"
AR_SCRIPT="$AR_SCRIPT"$'\n'"END"

echo "$AR_SCRIPT" | ar -M

make clean
