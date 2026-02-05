# { "Depends": "py-genlayer:test" }

import genlayer as gl
from genlayer.types import *
from genlayer.storage._internal.generate import generate_storage


@generate_storage
class Pr:
	x: gl.TreeMap[str, str]


a = Pr()

try:
	a.x = {'x': 'y'}
except AssertionError as e:
	print(*e.args)

exit(0)
