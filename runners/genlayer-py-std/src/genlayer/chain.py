__all__ = ('emit_transfer', 'Event', 'id')

import typing

from genlayer.types import Address, u256
from genlayer.contract import ON
import genlayer.calldata as calldata
import genlayer._internal.on_chain.gl_call as gl_call
from genlayer import IS_IN_VM


def emit_transfer(to: Address, /, amount: u256, *, on: ON = 'finalized') -> None:
	"""
	Emit transfer message to given address
	"""
	from genlayer.contract import get_at

	get_at(to).emit_transfer(value=amount, on=on)


from genlayer.types.keccak import Keccak256
from .vm import public_abi as ABI
from genlayer._internal import reflect
import inspect


class Event:
	# editorconfig-checker-disable
	"""
	.. code-block:: python

	        class TransferOccurredEvent(gl.Event):
	          def __init__(self, sender: Address, to: Address, /): ...


	        class TransferOccurredEvent(gl.Event):
	          def __init__(self, sender: Address, to: Address, /, **blob): ...
	"""

	# editorconfig-checker-enable

	def __init__(self):
		"""
		Base class cannot be instantiated directly, it is only used for inheritance
		"""
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
	def emit_raw(
		topics: list[bytes],
		blob: calldata.Encodable,
		/,
	) -> None:
		"""
		Emits a raw event with the given name, indexed fields and blob of data.
		"""
		gl_call.gl_call_generic(
			{
				'EmitEvent': {
					'topics': topics,
					'blob': blob,
				}
			},
			lambda _x: None,
		).get()

	@staticmethod
	def _do_init(klass) -> None:
		old_init = klass.__init__
		assert old_init is not Event.__init__

		klass.__slots__ = ('_blob',)

		sig = inspect.signature(old_init)

		indexed_args_lst: list[str] = []

		event_name = getattr(klass, 'name', klass.__name__)

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

		if len(indexed_args) > ABI.EVENT_MAX_TOPICS:
			import warnings

			warnings.warn(
				f'event has too many indexed fields, it may not be emitted correctly: `{signature}`'
			)

		klass.name = event_name
		klass.signature = signature
		klass.indexed = indexed_args
		klass.__init__ = __init__

	def __init_subclass__(cls) -> None:
		with reflect.context_notes('generating event class'):
			with reflect.context_type(cls):
				Event._do_init(cls)

	def emit(self) -> None:
		"""
		Emit this event.
		"""
		topics = [Keccak256(self.signature.encode('utf-8')).digest()]
		for i in self.indexed:
			d = self._blob[i]
			as_cd = calldata.encode(d)
			if len(as_cd) > 32:
				as_cd = Keccak256(as_cd).digest()
			else:
				as_cd = as_cd + b'\x00' * (32 - len(as_cd))
			topics.append(as_cd)

		Event.emit_raw(topics, self._blob)


if typing.TYPE_CHECKING or not IS_IN_VM:
	id: u256 = ...  # type: ignore
	"""
	Chain ID. It is the same as in the message
	"""
else:
	from genlayer.message import chain_id as id
