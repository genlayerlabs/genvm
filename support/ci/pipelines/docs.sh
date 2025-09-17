#!/usr/bin/env bash

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
source "$SCRIPT_DIR/_common.sh"

python3 ./runners/support/match-tags.py doc/website/src/impl-spec/appendix/runners-versions.json

nix develop -i .#py-test --command bash ./support/ci/pipelines/src/docs.sh
