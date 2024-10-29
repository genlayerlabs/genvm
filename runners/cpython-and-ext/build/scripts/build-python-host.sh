#!/usr/bin/env bash
set -ex

cd /opt/cpython

patch -p1 < /scripts/cpython.patch
perl -i -pe 's/pythonapi = /pythonapi = None #/g' ./Lib/ctypes/__init__.py

mkdir -p cross-build/build

pushd cross-build/build
../../configure --disable-test-modules --with-ensurepip=yes --prefix=/usr/

make -j
make install
make clean || true

popd

python3 --version
