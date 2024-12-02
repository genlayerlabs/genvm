"""
Common import for all contracts

It exposes most of the types to the top scope and encapsulates other utility under `gl` namespace which is a proxy to :py:mod:`genlayer.std`
"""

from .py.types import *
from .py.storage import *

if not typing.TYPE_CHECKING:

	class GL:
		"""
		proxy to :py:mod:`genlayer.std` used for lazy loading
		"""

		def __getattr__(self, attr):
			import genlayer.std as _imp

			# below is needed to trick cloudpickle
			global gl
			gl = _imp

			return getattr(_imp, attr)

	gl = GL()
	del GL
else:
	import genlayer.std as gl
