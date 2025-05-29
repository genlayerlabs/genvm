#!/usr/bin/env python3

import sys
from pathlib import Path
import hashlib

inp = sys.argv[1]
out = sys.argv[2]

in_dir = Path(inp)

for path in in_dir.iterdir():
	data = path.read_bytes()
	name = hashlib.sha3_256(data).digest().hex()
	Path(out).joinpath(name).write_bytes(data)
