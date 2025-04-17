#!/usr/bin/env bash

set -ex

git diff --ignore-all-space $1 $2 --stat -- \
    ':(exclude)runners/py-libs/pure-py/*' \
    ':(exclude)runners/py-libs/*' \
    ':(exclude)runners/nix/trg/softfloat/*' \
    '*.rb' '*.c' '*.h' '*.rs' '*.py' '*.lua' \
    '*.wat' '*.witx' \
    '*.toml' '*.json' '*.jsonnet' \
    '*.md'
