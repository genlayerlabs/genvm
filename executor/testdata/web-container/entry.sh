#!/usr/bin/env bash
set -ex

python3 /driver/server.py &
/usr/lib/jvm/java-11-openjdk-amd64/bin/java -jar /driver/selenium-server.jar standalone -I chrome --port 4444 &

wait -n
wait -n
