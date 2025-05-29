#!/usr/bin/env bash

set -ex

UPDATE_CORPUS=false
SHOW_HELP=false

show_help() {
    cat << EOF
Usage: $0 [OPTIONS]

OPTIONS:
    --help              Show this help message
    --update-corpus     Update input corpus
EOF
}

while [[ $# -gt 0 ]]; do
    case $1 in
        --help)
            SHOW_HELP=true
            shift
            ;;
        --update-corpus)
            UPDATE_CORPUS=true
            shift
        ;;
        *)
            echo "Error: Unknown option $1" >&2
            show_help
            exit 1
            ;;
    esac
done

if [[ $SHOW_HELP == true ]]; then
    show_help
    exit 0
fi


SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

cd "$SCRIPT_DIR/.."

for i in $(ls fuzz/src/*.py | sort)
do
    name="$(basename "$i" ".py")"
    echo "=== $name ==="
    mkdir -p "fuzz/outputs/$name"
    poetry run py-afl-fuzz -i "fuzz/inputs/$name" -o "fuzz/outputs/$name" -V 30 -- "fuzz/src/$name.py"


    if [[ "$UPDATE_CORPUS" == true ]]
    then
        rm -rf "fuzz/outputs/opt/$name" || true
        mkdir -p "fuzz/outputs/opt/$name"

        AFL_SKIP_BIN_CHECK=1 PYTHON_AFL_SIGNAL="SIGUSR1" \
            poetry run afl-cmin.bash \
            -T all \
            -o "fuzz/outputs/opt/$name" \
            -i "fuzz/outputs/$name" -- "./fuzz/src/$name.py"

        rm -rf "fuzz/inputs/$name" || true
        mkdir -p "fuzz/inputs/$name"
        poetry run ./fuzz/resave.py "fuzz/outputs/opt/$name" "fuzz/inputs/$name"
    fi
done
