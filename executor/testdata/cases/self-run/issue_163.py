# { "Depends": "py-genlayer:test" }

from genlayer import *
from genlayer.py.storage._internal.generate import storage


@storage
class Pr:
	x: TreeMap[str, str]


a = Pr()

try:
	a.x = {'x': 'y'}
except AssertionError as e:
	print(*e.args)

exit(0)
