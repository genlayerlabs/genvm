#!/usr/bin/env bash
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

set -ex

ls -lr /opt/wasi-sdk-24.0/bin
file /opt/wasi-sdk-24.0/bin/clang || true
/opt/wasi-sdk-24.0/bin/clang --version

bash "$SCRIPT_DIR/build-bz2.sh"
bash "$SCRIPT_DIR/build-lzma.sh"
bash "$SCRIPT_DIR/build-zlib.sh"
