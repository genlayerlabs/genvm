#!/usr/bin/env bash
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
source "/scripts/common.sh"

DIR="$1"
shift

find "$DIR" -type f -name '*.pyc' -delete

#wasmtime run \
#    --env PYTHONHOME=/py-std \
#    --dir /out/py/lib/python3.13::/py-std \
#    /out/py/bin/python3.13.wasm \
#        -m compileall \
#        --invalidation-mode unchecked-hash \
#        /py-std

python3 -m compileall \
    --invalidation-mode unchecked-hash \
    "$DIR"

chmod -R a+rw "$DIR"

if [ $# != 0 ]
then
    "$@"
fi
