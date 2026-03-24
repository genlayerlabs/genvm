import abc

from genlayer._internal.schema_attrs import (
	PUBLIC_ATTR as _PUBLIC_ATTR,
	READONLY_ATTR as _READONLY_ATTR,
	PAYABLE_ATTR as _PAYABLE_ATTR,
	MIN_GAS_LEADER_ATTR as _MIN_GAS_LEADER_ATTR,
	MIN_GAS_VALIDATOR_ATTR as _MIN_GAS_VALIDATOR_ATTR,
)


def private(f, /):
	"""
	Decorator that marks method as private. As all methods are private by default it does nothing.
	"""
	return f


class _payable(metaclass=abc.ABCMeta):
	def payable[T](self, f: T, /) -> T:
		self(f)
		setattr(f, _PAYABLE_ATTR, True)
		return f

	@abc.abstractmethod
	def __call__[T](self, f: T) -> T: ...


class _min_gas(_payable):
	__slots__ = ('_leader', '_validator')

	def __init__(self, leader: int, validator: int):
		self._leader = leader
		self._validator = validator

	def __call__[T](self, f: T) -> T:
		setattr(f, _PUBLIC_ATTR, True)
		setattr(f, _READONLY_ATTR, False)
		setattr(f, _MIN_GAS_LEADER_ATTR, self._leader)
		setattr(f, _MIN_GAS_VALIDATOR_ATTR, self._validator)
		return f


class _write(_payable):
	def min_gas(self, *, leader: int, validator: int) -> _min_gas:
		return _min_gas(leader, validator)

	def __call__[T](self, f: T) -> T:
		setattr(f, _PUBLIC_ATTR, True)
		setattr(f, _READONLY_ATTR, False)
		return f


class public:
	@staticmethod
	def view(f, /):
		"""
		Decorator that marks a contract method as a public view
		"""
		setattr(f, _PUBLIC_ATTR, True)
		setattr(f, _READONLY_ATTR, True)
		return f

	write = _write()
	"""
	Decorator that marks a contract method as a public write. Has `.payable`

	.. code:: python

		@gl.public.write
		def foo(self) -> None: ...

		@gl.public.write.payable
		def bar(self) -> None: ...

		@gl.public.write.min_gas(leader=100, validator=20).payable
		def bar(self) -> None: ...
	"""


del _write
