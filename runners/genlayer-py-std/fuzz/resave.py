#!/usr/bin/env python3

import sys
from pathlib import Path
import hashlib
import argparse

parser = argparse.ArgumentParser(
	description='Resave files with names as their sha3-256 hash'
)
parser.add_argument('input_dir', help='Directory containing input files')
parser.add_argument('output_dir', help='Directory to save output files')
args = parser.parse_args()

in_dir = Path(args.input_dir)
out_dir = Path(args.output_dir)

Path(out_dir).mkdir(parents=True, exist_ok=True)

for path in in_dir.iterdir():
	if path.is_dir():
		continue
	if path.name.startswith('.'):
		continue
	data = path.read_bytes()
	name = hashlib.sha3_256(data).digest().hex()
	Path(out_dir).joinpath(name).write_bytes(data)
