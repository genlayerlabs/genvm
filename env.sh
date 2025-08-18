#!/usr/bin/env bash

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

export PATH="$SCRIPT_DIR/tools/git-third-party:$PATH"

if [ -f "$SCRIPT_DIR/.env" ]
then
    set -a
    source "$SCRIPT_DIR/.env"
    set +a
fi
