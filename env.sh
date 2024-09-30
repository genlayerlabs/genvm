#!/usr/bin/env bash

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

PATH="$SCRIPT_DIR/tools/git-third-party:$SCRIPT_DIR/tools/ya-build:$PATH"
if [ -f "$SCRIPT_DIR/.env" ]
then
    source "$SCRIPT_DIR/.env"
fi
