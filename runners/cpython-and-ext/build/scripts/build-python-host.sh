#!/usr/bin/env bash
set -ex

cd /opt/cpython

patch -p1 <"/scripts/cpython.patch"

mkdir -p cross-build/build

pushd cross-build/build
../../configure --disable-test-modules --with-ensurepip=yes --prefix=/usr/

make -j
make install
make clean || true

popd

python3 --version
python3 -m pip install \
    --no-index --disable-pip-version-check --find-links /opt/whl \
    /opt/whl/Cython-3.0.11-cp313-cp313-manylinux_2_17_x86_64.manylinux2014_x86_64.whl
