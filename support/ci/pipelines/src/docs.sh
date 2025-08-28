#!/usr/bin/env bash

set -ex

pushd doc/website
poetry install --no-root
popd

mkdir -p build/doc

OUT_BASE="$(readlink -f build/doc)"

poetry run -C doc/website -- sphinx-build -b html "$(readlink -f doc/website/src)" "$OUT_BASE/html"
poetry run -C doc/website -- sphinx-build -b text "$(readlink -f doc/website/src)" "$OUT_BASE/text"

./doc/website/merge-txts.rb "$OUT_BASE/text/api" "$OUT_BASE/html/_static/ai/api.txt"
./doc/website/merge-txts.rb "$OUT_BASE/text/spec" "$OUT_BASE/html/_static/ai/spec.txt"
./doc/website/merge-txts.rb "$OUT_BASE/text/impl-spec" "$OUT_BASE/html/_static/ai/impl-spec.txt"
