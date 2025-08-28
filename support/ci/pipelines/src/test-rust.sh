#!/usr/bin/env bash

export PATH="$NIX:$PATH"

set -ex

ruby ./configure.rb

ninja -v -C build all/bin

LOGLEVEL=DEBUG python3 ./build/out/executor/vTEST/bin/post-install.py

bash ./tests/rust.sh
