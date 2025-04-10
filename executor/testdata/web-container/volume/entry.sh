#!/usr/bin/env bash
set -ex

python3 /test/server.py &
/src/start.sh &

wait -n
wait -n
