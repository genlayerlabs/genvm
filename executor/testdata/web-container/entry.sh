#!/usr/bin/env bash
set -ex

python3 -m http.server --directory /driver/http 80 &
/usr/lib/jvm/java-11-openjdk-amd64/bin/java -jar /driver/selenium-server.jar standalone -I chrome --port 4444 &

wait -n
wait -n
