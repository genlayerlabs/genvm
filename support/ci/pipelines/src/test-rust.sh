#!/usr/bin/env bash

export PATH="$NIX:$PATH"

set -ex

ruby ./configure.rb

ninja -v -C build all/bin

LOGLEVEL=DEBUG python3 ./build/out/bin/post-install.py --error-on-missing-executor=false

bash -x ./tests/rust.sh
