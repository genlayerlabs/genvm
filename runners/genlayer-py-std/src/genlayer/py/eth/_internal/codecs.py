import abc
import typing
import collections.abc

from functools import partial
from dataclasses import dataclass

from genlayer.py.types import Address
from genlayer.py.storage import Array

type Tails = list[typing.Callable[[EncodeState], None]]


@dataclass
class EncodeState:
	current_off: int
	result: bytearray
	tails: Tails

	def put_iloc(self):
		put_at = len(self.result)
		self.result.extend(b'\x00' * 32)

		off0 = self.current_off

		def putter(state):
			to_put = len(state.result) - off0
			self.result[put_at : put_at + 32] = int.to_bytes(to_put, 32, 'big')

		self.tails.append(putter)

	def derived(self) -> 'EncodeState':
		return EncodeState(
			current_off=len(self.result),
			result=self.result,
			tails=[],
		)

	def run_tails(self):
		current_tails = self.tails
		self.tails = []
		while len(current_tails) != 0:
			for i in current_tails:
				i(self)
			current_tails = self.tails
			self.tails = []


@dataclass
class DecoderState:
	mem: memoryview
	current_off: int
	current_off_0: int

	def fetch_head(self, le: int) -> memoryview:
		res = self.mem[self.current_off : self.current_off + le]
		self.current_off += le
		return res

	def indirected(self) -> 'DecoderState':
		off = int.from_bytes(self.fetch_head(32), 'big', signed=False)

		new_off_0 = self.current_off_0 + off
		return DecoderState(
			current_off_0=new_off_0,
			current_off=new_off_0,
			mem=self.mem,
		)


class Encoder[T](metaclass=abc.ABCMeta):
	@property
	@abc.abstractmethod
	def is_dynamic(self) -> bool: ...

	@property
	@abc.abstractmethod
	def name(self) -> str: ...

	@property
	@abc.abstractmethod
	def size_here(self) -> int: ...

	@abc.abstractmethod
	def encode(self, state: EncodeState, val: T): ...

	@abc.abstractmethod
	def decode(self, state: DecoderState) -> T: ...

	def __repr__(self):
		return self.name


class IntEncoder[T: int](Encoder[T]):
	@property
	def is_dynamic(self) -> bool:
		return False

	@property
	def size_here(self) -> int:
		return 32

	@property
	def name(self) -> str:
		return self._name

	def __init__(self, bits: int, signed: bool):
		if signed:
			self._name = f'int{bits}'
		else:
			self._name = f'uint{bits}'
		self.signed = signed

	def encode(self, state: EncodeState, val: T):
		state.result.extend(val.to_bytes(32, 'big', signed=self.signed))

	def decode(self, state: DecoderState) -> int:  # type: ignore
		return int.from_bytes(state.fetch_head(32), 'big', signed=self.signed)


class BoolEncoder(Encoder[bool]):
	@property
	def is_dynamic(self) -> bool:
		return False

	@property
	def size_here(self) -> int:
		return 32

	@property
	def name(self) -> str:
		return 'bool'

	def encode(self, state: EncodeState, val: bool):
		state.result.extend(int.to_bytes(1 if val else 0, 32, 'big'))

	def decode(self, state: DecoderState) -> bool:
		return state.fetch_head(32) != b'\x00' * 32


class AddressEncoder(Encoder[Address]):
	@property
	def is_dynamic(self) -> bool:
		return False

	@property
	def size_here(self) -> int:
		return 32

	@property
	def name(self) -> str:
		return 'address'

	def encode(self, state: EncodeState, val: Address):
		state.result.extend(b'\x00' * 12)
		state.result.extend(val.as_bytes)

	def decode(self, state: DecoderState) -> Address:
		state.fetch_head(12)
		return Address(state.fetch_head(20))


class BytesN(Encoder):
	def __init__(self, bytes: int):
		self.bytes = bytes

	@property
	def is_dynamic(self) -> bool:
		return False

	@property
	def size_here(self) -> int:
		return 32

	@property
	def name(self) -> str:
		return f'bytes{self.bytes}'

	def encode(self, state: EncodeState, val):
		assert len(val) == self.bytes
		state.result.extend(val)
		state.result.extend(b'\x00' * (32 - self.bytes))

	def decode(self, state: DecoderState):
		res = state.fetch_head(self.bytes)
		state.fetch_head(32 - self.bytes)
		return res


class BytesStrEncoder[T: str | bytes](Encoder[T]):
	def __init__(self, t: typing.Type[T]):
		self.type = t

	@property
	def is_dynamic(self) -> bool:
		return True

	@property
	def size_here(self) -> int:
		return 32

	@property
	def name(self) -> str:
		if issubclass(self.type, str):
			return 'string'
		else:
			return 'bytes'

	def encode(self, state: EncodeState, val: T):
		state.put_iloc()
		as_bytes: bytes
		if issubclass(self.type, str):
			as_bytes = val.encode('utf-8')  # type: ignore
		else:
			as_bytes = val  # type: ignore

		def put_bytes(state):
			state.result.extend(int.to_bytes(len(as_bytes), 32, 'big'))
			state.result.extend(as_bytes)
			state.result.extend(b'\x00' * ((32 - len(as_bytes) % 32) % 32))

		state.tails.append(put_bytes)

	def decode(self, state: DecoderState) -> T:
		state = state.indirected()

		le = int.from_bytes(state.fetch_head(32), 'big', signed=False)
		as_bytes = state.fetch_head(le)
		if issubclass(self.type, str):
			return str(as_bytes, 'utf-8')  # type: ignore
		else:
			return bytes(as_bytes)  # type: ignore


class DynArrayEncoder[T](Encoder[collections.abc.Sequence[T]]):
	def __init__(self, elem_encoder: Encoder[T]):
		self.elem_encoder = elem_encoder

	@property
	def is_dynamic(self) -> bool:
		return True

	@property
	def size_here(self) -> int:
		return 32

	@property
	def name(self) -> str:
		return self.elem_encoder.name + '[]'

	def _encode_now(self, state: EncodeState, val: collections.abc.Sequence[T]):
		state.result.extend(int.to_bytes(len(val), 32, 'big'))

		der = state.derived()
		for v in val:
			der.tails.append(partial(self.elem_encoder.encode, val=v))
		der.run_tails()

	def encode(self, state: EncodeState, val: collections.abc.Sequence[T]):
		state.put_iloc()
		state.tails.append(partial(self._encode_now, val=val))

	def decode(self, state: DecoderState) -> collections.abc.Sequence[T]:
		state = state.indirected()
		le = int.from_bytes(state.fetch_head(32), 'big', signed=False)
		state.current_off_0 += 32
		res = []
		for i in range(le):
			res.append(self.elem_encoder.decode(state))
		return res


class ArrayEncoder[T, S: int](Encoder[Array[T, S]]):
	def __init__(self, elem_encoder: Encoder[T], elem_count: S):
		self.elem_encoder = elem_encoder
		self.elem_count = elem_count

	@property
	def is_dynamic(self) -> bool:
		return False

	@property
	def size_here(self) -> int:
		return self.elem_encoder.size_here * self.elem_count

	@property
	def name(self) -> str:
		return self.elem_encoder.name + f'[{self.elem_count}]'

	def encode(self, state: EncodeState, val: Array[T, S]):
		for v in val:
			self.elem_encoder.encode(state, v)

	def decode(self, state: DecoderState) -> Array[T, S]:
		res = []
		for i in range(self.elem_count):
			res.append(self.elem_encoder.decode(state))
		return res  # type: ignore


class TupleEncoder[*T](Encoder[tuple[*T]]):
	def __init__(self, elem_encoders: tuple[Encoder, ...], force_inplace: bool):
		self.elem_encoders = elem_encoders
		if force_inplace:
			self._is_dynamic = False
		else:
			self._is_dynamic = any(e.is_dynamic for e in elem_encoders)
		if self._is_dynamic:
			self._size_here = 32
		else:
			self._size_here = sum(e.size_here for e in elem_encoders)

		self._name = '(' + ','.join(e.name for e in elem_encoders) + ')'

	@property
	def is_dynamic(self) -> bool:
		return self._is_dynamic

	@property
	def size_here(self) -> int:
		return self._size_here

	@property
	def name(self) -> str:
		return self._name

	def _encode_now(self, state: EncodeState, val: tuple[*T]):
		if self._is_dynamic:
			der = state.derived()
		else:
			der = state
		for p, a in zip(self.elem_encoders, val):
			der.tails.append(partial(p.encode, val=a))
		der.run_tails()

	def encode(self, state: EncodeState, val: tuple[*T]):
		assert len(val) == len(self.elem_encoders)

		if self._is_dynamic:
			state.put_iloc()
			state.tails.append(partial(self._encode_now, val=val))
		else:
			self._encode_now(state, val)

	def decode(self, state: DecoderState) -> tuple[*T]:
		res = []
		if self._is_dynamic:
			state = state.indirected()

		for enc in self.elem_encoders:
			res.append(enc.decode(state))
		return tuple(res)
