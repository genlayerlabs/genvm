#!/usr/bin/env bash

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
source "$SCRIPT_DIR/_common.sh"

COPYRIGHT_YEAR="$(git log -1 --date=format:%Y --format=%ad)"
export COPYRIGHT_YEAR

python3 ./doc/website/generate.py doc/website/src/impl-spec/appendix/runners-versions.json

nix develop -i .#gen-docs --command bash ./support/ci/pipelines/src/docs.sh
