import inspect


def try_get_lineno(m):
	try:
		origin = inspect.getsourcefile(m)
	except Exception:
		origin = '<unknown>'
	try:
		_, lineno = inspect.findsource(m)
	except Exception:
		lineno = '?'
	return {'origin': origin, 'line': lineno}
