#!/usr/bin/env bash

set -ex

cd ./runners/genlayer-py-std

poetry install --no-root

poetry run -- pytest
