#/bin/bash
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

set -ex

bash "$SCRIPT_DIR/build-bz2.sh"
bash "$SCRIPT_DIR/build-lzma.sh"
bash "$SCRIPT_DIR/build-zlib.sh"
bash "$SCRIPT_DIR/build-python-host.sh"
bash "$SCRIPT_DIR/build-python.sh"

#cd /opt/cpython/cross-build/wasm32-wasi
#find Programs Python/ -type f -name '*.o' | sort | xargs sha256sum
#find /opt/wasm32-wasip1-root/lib/ -type f | sort | xargs sha256sum
sha256sum /opt/cpython/cross-build/wasm32-wasi/python.wasm
