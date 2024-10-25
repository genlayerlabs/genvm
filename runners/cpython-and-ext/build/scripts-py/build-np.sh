#!/usr/bin/env bash
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
source "/scripts/common.sh"

cd /opt/numpy

python3 vendored-meson/meson/meson.py setup --cross-file /scripts-py/numpy/cross-file.txt build-wasm --prefix /opt/np-built

cd build-wasm
python3 ../vendored-meson/meson/meson.py install --tags runtime,python-runtime

perl -i -pe 's/"args": r".*",/"args": r"",/' /opt/np-built/lib/python3.13/site-packages/numpy/__config__.py

cp /scripts-py/numpy/_multiarray_umath.py /opt/np-built/lib/python3.13/site-packages/numpy/_core/_multiarray_umath.py
cp /scripts-py/numpy/_umath_linalg.py /opt/np-built/lib/python3.13/site-packages/numpy/linalg/_umath_linalg.py

AR_SCRIPT="CREATE /opt/np-built/all.a"
for f in $(find /opt/np-built/lib/python3.13/site-packages/numpy -name '*.so' | sort)
do
    AR_SCRIPT="$AR_SCRIPT"$'\n'"ADDLIB $f"
done

find /opt/np-built/lib/python3.13/site-packages/numpy -name '*.so' | sort | xargs sha256sum >> /out/checksums

AR_SCRIPT="$AR_SCRIPT"$'\n'"SAVE"
AR_SCRIPT="$AR_SCRIPT"$'\n'"END"

echo "$AR_SCRIPT" | ar -M

sha256sum /opt/np-built/all.a >> /out/checksums
