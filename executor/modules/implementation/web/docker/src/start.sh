#!/usr/bin/env bash
set -ex

#geckodriver --port 4444 --host 0.0.0.0 --allow-hosts localhost

ls /download

exec java -jar /wd/selenium-server.jar standalone -I firefox --port "${PORT:-4444}" --host 0.0.0.0
