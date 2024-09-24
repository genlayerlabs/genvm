#/bin/bash
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
source "$SCRIPT_DIR/common.sh"

cd /opt/xz-5.6.2

C_AND_CXX_FLAGS="-O3 --sysroot=/opt/wasi-sdk-24.0/share/wasi-sysroot --target=wasm32-wasi "
env \
    CC=/opt/wasi-sdk-24.0/bin/clang \
    CXX=/opt/wasi-sdk-24.0/bin/clang++ \
    CFLAGS="$C_AND_CXX_FLAGS" \
    CXXFLAGS="$C_AND_CXX_FLAGS" \
    ./configure \
        "--prefix=$WASM32_WASI_ROOT" \
        --host=wasm32-wasi \
        --enable-threads=no --enable-small --enable-decoders=lzma1,lzma2 \
        --disable-scripts --disable-doc

make -C src/liblzma/ -j
make -C src/liblzma/ install
