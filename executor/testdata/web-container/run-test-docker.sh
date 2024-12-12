#!/usr/bin/env bash
set -ex
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

cd "$SCRIPT_DIR"
IMAGE_ID="$(docker build -q -t genvm/modules-webdriver -f ./webdriver.dockerfile .)"
docker run \
    --add-host genvm-test:127.0.0.1 \
    -p 4444:4444 \
    --rm \
    --name genvm-web-test \
    --volume ./http:/driver/http \
    "$IMAGE_ID"
