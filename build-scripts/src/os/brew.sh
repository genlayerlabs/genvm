#!/usr/bin/env bash
set -ex
brew install ninja curl git tree pkg-config git-lfs
brew install ruby@3.2
brew install python@3.12
git lfs install
