def __getattr__(name):
	import bit_generator as _imp

	return getattr(_imp, name)
