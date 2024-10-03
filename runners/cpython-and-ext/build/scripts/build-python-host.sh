#!/usr/bin/env bash
set -ex

cd /opt/cpython

patch -p1 <"/scripts/cpython.patch"

mkdir -p cross-build/build

pushd cross-build/build
../../configure --disable-test-modules --with-ensurepip=no --prefix=/usr/

make -j
make install

popd

python3 --version
