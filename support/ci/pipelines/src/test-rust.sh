#!/usr/bin/env bash

set -ex

./configure.rb

ninja -v -C build all

LOGLEVEL=DEBUG ./build/out/executor/vTEST/bin/post-install.py

./tests/rust.sh
