from .py.types import *
from .py.storage import *


def make_gl():
	class GL:
		def __getattr__(self, attr):
			import genlayer.std as _imp

			# below is needed to trick cloudpickle
			global gl
			gl = _imp

			return getattr(_imp, attr)

	return GL()


if not typing.TYPE_CHECKING:
	gl = make_gl()
else:
	import genlayer.std as gl

	gl = gl

del make_gl
