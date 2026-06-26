#!/usr/bin/env bash

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
source "$SCRIPT_DIR/_common.sh"

export ORIGINAL_PATH="$PATH"
export ORIGINAL_LD_LIBRARY_PATH="$LD_LIBRARY_PATH"

mkdir -p build/out/executor/vTEST/data

# gvm32 (Crockford Base32) is the encoding the executor uses for runner paths;
# it is carried in `x.uid` as `id:gvm32hash`. Do NOT use `toHashFormat = "nix32"`
# here: Nix base32 has a different alphabet, so the registry names would never
# match the on-disk tars / GCS objects (e.g. nix32 `04l343…` vs gvm32 `5tnhg…`).
nix eval --verbose --impure --read-only --show-trace --json --expr \
    'let drv = import ./runners { host-system = builtins.currentSystem; } ; in builtins.listToAttrs (builtins.map (x: { name = x.id; value = builtins.head (builtins.match "[^:]+:(.*)" x.uid); }) drv)' \
    > build/out/executor/vTEST/data/latest.json

nix eval --verbose --impure --read-only --show-trace --json --expr \
    'let drv = import ./runners { host-system = builtins.currentSystem; } ; in builtins.listToAttrs (builtins.map (x: { name = x.id; value = [ (builtins.head (builtins.match "[^:]+:(.*)" x.uid)) ]; }) drv)' \
    > build/out/executor/vTEST/data/all.json

# we can't run it within nix because it uses `nix add` which sigsegvs
python3 ./support/runner-script.py \
    download \
    --nix-preload --allow-partial --dest build/out/runners --registry build/out/executor/vTEST/data/all.json

nix build -v -L -o build/out-runners .#debug-runners
mkdir -p ./build/out/runners
cp -r ./build/out-runners/. ./build/out/runners/.
chmod -R +w ./build/out/runners/.

nix develop .#rust-test --command python3 \
    ./support/runner-script.py \
    upload \
    --root build/out/runners --registry build/out/executor/vTEST/data/all.json || true

nix develop .#rust-test --command bash ./support/ci/pipelines/src/test-rust.sh
