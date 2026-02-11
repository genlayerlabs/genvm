#!/usr/bin/env bash

export PATH="$NIX:$PATH"

set -ex

ruby ./configure.rb

nix develop .#mock-tests --command ya-test-runner --test-tags "$(cat tests/presets/rust-fuzz.txt)" run
