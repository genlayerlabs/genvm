from genlayer.py.get_schema import get_schema


class A:
	pass


def test_no_init():
	try:
		get_schema(A)
	except TypeError:
		pass
	else:
		assert False
