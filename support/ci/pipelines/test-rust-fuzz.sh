#!/usr/bin/env bash

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
source "$SCRIPT_DIR/_common.sh"

export ORIGINAL_PATH="$PATH"
export ORIGINAL_LD_LIBRARY_PATH="$LD_LIBRARY_PATH"

nix develop .#rust-test --command bash ./support/ci/pipelines/src/test-rust-fuzz.sh
