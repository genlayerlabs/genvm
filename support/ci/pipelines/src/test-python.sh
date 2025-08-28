#!/usr/bin/env bash

set -ex

cd ./runners/genlayer-py-std

poetry install --with dev

poetry run -- pytest
