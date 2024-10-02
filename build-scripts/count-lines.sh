#!/bin/bash

set -ex

git diff $1 $2 --stat -- \
    ':(exclude)runners/genlayer-std/src/cloudpickle/*' \
    '*.rb' '*.c' '*.h' '*.rs' '*.py' \
    '*.wat' '*.witx' \
    '*.toml' '*.json' '*.jsonnet' \
    '*.md'
