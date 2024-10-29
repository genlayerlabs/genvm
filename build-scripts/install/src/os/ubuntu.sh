#!/usr/bin/env bash
set -ex
sudo apt-get install -y pkg-config ninja-build curl git python3 zip unzip tar tree mold
sudo apt-get satisfy -y 'ruby (>= 3.0)'
git lfs install
