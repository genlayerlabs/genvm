#!/usr/bin/env bash
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

set -ex

ls -lr /opt/host-root/bin
/opt/host-root/bin/clang --version

bash "$SCRIPT_DIR/build-bz2.sh"
bash "$SCRIPT_DIR/build-lzma.sh"
bash "$SCRIPT_DIR/build-zlib.sh"
bash "$SCRIPT_DIR/build-ffi.sh"
