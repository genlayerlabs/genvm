#!/usr/bin/env bash

export PATH="$NIX:$PATH"

set -ex

ruby ./configure.rb

nix develop .#mock-tests --command ya-test-runner --filter-tag "$(cat tests/presets/rust-fuzz.txt)" run
