#!/usr/bin/env bash
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
source "/scripts/common.sh"

DIR="$1"
shift

find "$DIR" -type f -name '*.pyc' -delete

python3 -m compileall \
    --invalidation-mode unchecked-hash \
    "$DIR"

chmod -R a+rw "$DIR"

if [ $# != 0 ]
then
    "$@"
fi
