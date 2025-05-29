#!/usr/bin/env python3

import genlayer.py.calldata as calldata

import sys
from pathlib import Path

sys.path.append(str(Path(__file__).parent.parent))
from fuzz_common import do_fuzzing, StopFuzzingException, FuzzerBuilder


def calldata_decoding(buf):
	try:
		decoded = calldata.decode(buf)
	except (calldata.DecodingError, UnicodeDecodeError):
		return
	got = calldata.encode(decoded)

	assert got == buf, f'decoded is `{decoded}`'


if __name__ == '__main__':
	do_fuzzing(calldata_decoding)
