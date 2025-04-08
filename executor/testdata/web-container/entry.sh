#!/usr/bin/env bash
set -ex

python3 /driver/server.py &
chromedriver --port=4444 --whitelisted-ips &

wait -n
wait -n
