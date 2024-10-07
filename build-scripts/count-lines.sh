#!/usr/bin/env bash

set -ex

git diff $1 $2 --stat -- \
    ':(exclude)runners/genlayer-py-std/src/cloudpickle/*' \
    '*.rb' '*.c' '*.h' '*.rs' '*.py' \
    '*.wat' '*.witx' \
    '*.toml' '*.json' '*.jsonnet' \
    '*.md'
