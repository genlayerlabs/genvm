__all__ = ('Event',)

import inspect
import genlayer.py.calldata as calldata
import genlayer.py._internal.reflect as reflect


class Event:
	"""
	.. code-block:: python

		class TransferOccurredEvent(gl.Event):
			def __init__(self, from: Address, to: Address, /): ...

		class TransferOccurredEvent(gl.Event):
			def __init__(self, from: Address, to: Address, /, **blob): ...
	"""

	def __init__(self):
		raise NotImplementedError()

	signature: str
	"""
	Event signature (built-in/main topic). Consists of name and indexed fields in parenthesis, **sorted**

	Example: ``TransferOccurredEvent(from,to)``
	"""
	name: str
	"""
	Event name. If not overridden it will be set to ``__name__``
	"""
	indexed: tuple[str, ...]
	"""
	tuple of indexed arguments name in **sorted** order
	"""

	_blob: dict[str, calldata.Encodable]

	__slots__ = ('_blob',)

	@staticmethod
	def _do_init(cls) -> None:
		old_init = cls.__init__
		assert old_init is not Event.__init__

		cls.__slots__ = ('_blob',)

		sig = inspect.signature(old_init)

		indexed_args_lst: list[str] = []

		event_name = getattr(cls, 'name', cls.__name__)

		for i, (name, param) in enumerate(sig.parameters.items()):
			with reflect.context_notes(f'parameter `{name}`'):
				if i == 0:
					if name != 'self':
						raise TypeError('first argument must be `self`')
					continue

				match param.kind:
					case inspect.Parameter.VAR_POSITIONAL:
						raise TypeError('`*args` is forbidden')
					case inspect.Parameter.KEYWORD_ONLY:
						raise TypeError('keyword-only arguments are forbidden')
					case inspect.Parameter.POSITIONAL_OR_KEYWORD:
						raise TypeError('specify `/` after indexed fields')
					case inspect.Parameter.VAR_KEYWORD:
						pass
					case inspect.Parameter.POSITIONAL_ONLY:
						indexed_args_lst.append(name)

		indexed_args = tuple(sorted(indexed_args_lst))

		def __init__(self, *args, **kwargs):
			if len(args) != len(indexed_args):
				raise TypeError(
					f'indexed fields mismatch, expected {indexed_args}, but got {len(args)} positional arguments'
				)

			for name, val in zip(indexed_args, args):
				if name in kwargs:
					raise TypeError(f'indexed field `{name}` must not be present in blob')
				kwargs[name] = val

			self._blob = kwargs

		signature = event_name
		signature += '('
		signature += ','.join(indexed_args)
		signature += ')'

		cls.name = event_name
		cls.signature = signature
		cls.indexed = indexed_args
		cls.__init__ = __init__

	def __init_subclass__(cls) -> None:
		with reflect.context_notes('generating event class'):
			with reflect.context_type(cls):
				Event._do_init(cls)

	def emit(self) -> None:
		"""
		emit this event
		"""
		...
