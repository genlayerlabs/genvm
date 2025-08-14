#!/usr/bin/env bash
set -ex

# Run the built Puppeteer application
cd /src/prj
exec node dist/index.js --port "${PORT:-4444}"
