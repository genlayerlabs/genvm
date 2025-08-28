#!/usr/bin/env bash

set -ex

./configure.rb

ninja -v -C build out/executor/vTEST/data/all.json

./support/runner-script.py download --nix-preload --allow-partial --dest build/out/runners --registry build/out/executor/vTEST/data/all.json

ninja -v -C build all

./support/runner-script.py upload --root build/out/runners --registry build/out/executor/vTEST/data/all.json || true

LOGLEVEL=DEBUG ./build/out/executor/vTEST/bin/post-install.py

./tests/rust.sh
