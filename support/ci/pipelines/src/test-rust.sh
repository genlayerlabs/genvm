#!/usr/bin/env bash

export PATH="$NIX:$PATH"

set -ex

ruby ./configure.rb

ninja -v -C build all/bin

python3 ./build/out/bin/post-install.py \
    --error-on-missing-executor=false \
    --default-download=false

bash -x ./tests/rust.sh
