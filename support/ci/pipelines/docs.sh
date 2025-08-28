#!/usr/bin/env bash

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
source "$SCRIPT_DIR/_common.sh"

nix develop .#py-test --command bash ./support/ci/pipelines/src/docs.sh
