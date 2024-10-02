#!/usr/bin/env bash

find "$@" -type f -not -name '*.d' -and -not -name 'root-output' | sort | xargs sha256sum
