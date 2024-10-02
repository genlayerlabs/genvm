#!/usr/bin/bash
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

set -ex

bash "$SCRIPT_DIR/build-bz2.sh"
bash "$SCRIPT_DIR/build-lzma.sh"
bash "$SCRIPT_DIR/build-zlib.sh"
