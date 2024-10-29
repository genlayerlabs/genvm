#!/usr/bin/env bash

set -ex

git diff --ignore-all-space $1 $2 --stat -- \
    ':(exclude)runners/genlayer-py-std/src/cloudpickle/*' \
    ':(exclude)runners/py-libs/*' \
    '*.rb' '*.c' '*.h' '*.rs' '*.py' \
    '*.wat' '*.witx' \
    '*.toml' '*.json' '*.jsonnet' \
    '*.md'
