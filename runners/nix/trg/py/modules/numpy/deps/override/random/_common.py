def __getattr__(name):
	import _common as _imp

	return getattr(_imp, name)
