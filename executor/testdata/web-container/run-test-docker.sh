#!/usr/bin/env bash
set -ex
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )

cd "$SCRIPT_DIR"

DOCKER_BUILD_DIR=../../modules/implementation/webdriver

IMAGE_ID="$(docker build -q -t genvm/modules-webdriver "$DOCKER_BUILD_DIR")"

docker run \
    --add-host genvm-test:127.0.0.1 \
    -p 4444:4444 \
    -p 4445:80 \
    --rm -d \
    --name genvm-web-test \
    --volume ./volume:/test/ \
    "$IMAGE_ID" \
    bash /test/entry.sh

echo "remember to run" perl -pe 's/(always_allow_hosts:) \[\]/$1 ["localhost"]/' -i ./build/out/config/genvm-module-web.yaml
