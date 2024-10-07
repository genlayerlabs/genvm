#!/usr/bin/env bash
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
source "/scripts/common.sh"

cd "$1"
shift

NAME="$1"
shift

zip -r "$NAME" "$@"
