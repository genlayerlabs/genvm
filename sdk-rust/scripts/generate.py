#!/bin/env python3

import os
from pathlib import Path
import re

script_dir = Path(__file__).parent

generated_file =script_dir.joinpath('tmp', 'genlayer_sdk.witx')

generated_file.parent.mkdir(parents=True, exist_ok=True)

files_dir = script_dir.parent.parent.joinpath(*'genvm/src/wasi/witx/'.split('/'))
with open(files_dir.joinpath('genlayer_sdk.witx')) as f:
    contents = f.read()

with open(files_dir.joinpath('genlayer_sdk_types.witx')) as f:
    types = f.read()

contents = contents.replace('(use "genlayer_sdk_types.witx")', types)
#contents = contents.replace(';; TYPES_IN_MODULE', re.sub(r'^', '    ', types, flags=re.MULTILINE))

with open(generated_file, 'wt') as f:
    f.write(contents)

import subprocess
out = subprocess.check_output(["cargo", "run", "-q", generated_file], cwd=script_dir.parent.joinpath('third-party', 'wasi-rs', 'crates', 'witx-bindgen'), encoding='utf8')

with open(script_dir.parent.joinpath('src', 'generated.rs'), 'wt') as f:
    f.write(out)

#subprocess.run(["witx-codegen", "-o", script_dir.parent.joinpath('src', 'lib.rs'), '--output-type', 'rust', generated_file])
