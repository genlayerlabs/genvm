"""
Module that provides eth calldata encoding and decoding
"""

__all__ = ('MethodEncoder', 'decode')
from ..keccak import Keccak256

import typing

from genlayer.py.types import *
from genlayer.py.storage import Array, DynArray
from functools import partial

_integer_types = {
	u8: 'uint8',
	u16: 'uint16',
	u24: 'uint24',
	u32: 'uint32',
	u40: 'uint40',
	u48: 'uint48',
	u56: 'uint56',
	u64: 'uint64',
	u72: 'uint72',
	u80: 'uint80',
	u88: 'uint88',
	u96: 'uint96',
	u104: 'uint104',
	u112: 'uint112',
	u120: 'uint120',
	u128: 'uint128',
	u136: 'uint136',
	u144: 'uint144',
	u152: 'uint152',
	u160: 'uint160',
	u168: 'uint168',
	u176: 'uint176',
	u184: 'uint184',
	u192: 'uint192',
	u200: 'uint200',
	u208: 'uint208',
	u216: 'uint216',
	u224: 'uint224',
	u232: 'uint232',
	u240: 'uint240',
	u248: 'uint248',
	u256: 'uint256',
	i8: 'int8',
	i16: 'int16',
	i24: 'int24',
	i32: 'int32',
	i40: 'int40',
	i48: 'int48',
	i56: 'int56',
	i64: 'int64',
	i72: 'int72',
	i80: 'int80',
	i88: 'int88',
	i96: 'int96',
	i104: 'int104',
	i112: 'int112',
	i120: 'int120',
	i128: 'int128',
	i136: 'int136',
	i144: 'int144',
	i152: 'int152',
	i160: 'int160',
	i168: 'int168',
	i176: 'int176',
	i184: 'int184',
	i192: 'int192',
	i200: 'int200',
	i208: 'int208',
	i216: 'int216',
	i224: 'int224',
	i232: 'int232',
	i240: 'int240',
	i248: 'int248',
	i256: 'int256',
}

_simple = {
	bool: 'bool',
	str: 'string',
	bytes: 'bytes',
	Address: 'address',
	**_integer_types,
}


def get_type_eth_name(t: type) -> str:
	if (simp := _simple.get(t, None)) is not None:
		return simp

	origin = typing.get_origin(t)
	if origin is not None:
		args = typing.get_args(t)
		if origin is Array:
			assert typing.get_origin(args[1]) is typing.Literal
			le = int(*typing.get_args(args[1]))
			if args[0] == u8 and le >= 1 and le <= 32:
				return f'bytes{le}'
			else:
				return f'{get_type_eth_name(args[0])}[{le}]'
		elif origin is list or origin is DynArray:
			assert len(args) == 1
			return f'{get_type_eth_name(args[0])}[]'
		elif origin is tuple:
			return '(' + ','.join(map(get_type_eth_name, args)) + ')'
	assert False, f'unknown type {t} {type(t)} {t is str} {type(t) is str}'


def is_dynamic(param: type):
	if param is bytes or param is str:
		return True
	origin = typing.get_origin(param)
	if origin is None:
		return False
	type_args = typing.get_args(param)
	if origin is DynArray or origin is list:
		return True
	elif origin is tuple:
		return any(is_dynamic(x) for x in type_args)
	elif origin is Array:
		return True
	return False


def calc_size_here(param: type) -> int:
	if is_dynamic(param):
		return 32
	if param in _integer_types:
		return 32
	return False


type _Tails = list[typing.Callable[[_Tails], None]]


class MethodEncoder:
	"""
	Type used to encode method call
	"""

	name: str
	"""
	method name
	"""

	params: list[type]
	"""
	method parameter types
	"""

	ret: type
	"""
	return type (can be unused)
	"""

	selector: bytes
	"""
	calculated function "selector", see eth docs
	"""

	__slots__ = ('name', 'params', 'ret', 'selector')

	def __init__(self, name: str, params: list[type], ret: type):
		self.name = name
		self.params = params
		self.ret = ret
		sig = self.make_sig()
		self.selector = Keccak256(sig.encode('utf-8')).digest()[:4]

	def make_sig(self) -> str:
		"""
		calculates signature that is used for making method selector
		"""
		sig: list[str] = [self.name, '(']
		for i, par in enumerate(self.params):
			if i != 0:
				sig.append(',')
			sig.append(get_type_eth_name(par))
		sig.append(')')
		return ''.join(sig)

	def encode(self, args: list[typing.Any]) -> bytes:
		"""
		encodes ``args`` according to this encoder ``params`` to produce a calldata

		:returns: full calldata encoded call: both selector and arguments
		"""
		assert len(args) == len(self.params)

		result: bytearray = bytearray()
		result.extend(self.selector)

		current_off: int = len(result)

		def run_seq_with_new_tails(cur: _Tails):
			nonlocal current_off
			old_off = current_off
			current_off = len(result)
			loc_tails: _Tails = []
			while len(cur) != 0:
				for i in cur:
					i(loc_tails)
				cur = loc_tails
				loc_tails = []
			current_off = old_off

		def put_offset_at(off: int, off0: int) -> None:
			to_put = len(result) - off0
			memoryview(result)[off : off + 32] = int.to_bytes(to_put, 32, 'big')

		def put_iloc(tails: _Tails):
			off = len(result)
			result.extend(b'\x00' * 32)
			tails.append(lambda _t: put_offset_at(off, current_off))

		def put_regular(param: type, arg: typing.Any, tails: _Tails) -> None:
			as_int = _integer_types.get(param, None)
			if as_int is not None:
				result.extend(int.to_bytes(arg, 32, 'big', signed=as_int.startswith('i')))
			elif param is bool:
				result.extend(int.to_bytes(1 if arg else 0, 32, 'big'))
			elif param is Address:
				result.extend(b'\x00' * 12)
				result.extend(arg.as_bytes)
			elif param is bytes or param is str:
				put_iloc(tails)
				if param is bytes:
					as_bytes = typing.cast(bytes, arg)
				else:
					as_bytes = typing.cast(str, arg).encode('utf-8')

				def put_bytes(_tails):
					result.extend(int.to_bytes(len(as_bytes), 32, 'big'))
					result.extend(as_bytes)
					result.extend(b'\x00' * ((32 - len(as_bytes) % 32) % 32))

				tails.append(put_bytes)
			elif (origin := typing.get_origin(param)) is not None:
				type_args = typing.get_args(param)
				if origin is DynArray or origin is list:
					put_iloc(tails)
					as_seq = typing.cast(collections.abc.Sequence, arg)

					def put_arr(tails: _Tails):
						result.extend(int.to_bytes(len(as_seq), 32, 'big'))
						cur: _Tails = []
						for i in range(len(as_seq)):
							cur.append(partial(put_regular, type_args[0], as_seq[i]))
						run_seq_with_new_tails(cur)

					tails.append(put_arr)
				elif origin is Array:
					assert typing.get_origin(type_args[1]) is typing.Literal
					le = int(*typing.get_args(type_args[1]))
					for i in range(le):
						put_regular(type_args[0], arg[i], tails)
				elif origin is tuple:

					def put_tuple(_tails):
						cur: _Tails = []
						for p, a in zip(type_args, arg):
							cur.append(partial(put_regular, p, a))
						run_seq_with_new_tails(cur)

					if is_dynamic(param):
						put_iloc(tails)
						tails.append(put_tuple)
					else:
						put_tuple(None)
				else:
					assert False
			else:
				assert False

		cur: _Tails = []
		for p, a in zip(self.params, args):
			cur.append(partial(put_regular, p, a))

		run_seq_with_new_tails(cur)

		return bytes(result)


def decode(params: list[type], data: collections.abc.Buffer) -> list[typing.Any]:
	"""
	:param params: eth method returns (if it returns a single value, i.e. ``u32``, provide a list of one element)
	:param data: eth calldata encoded structure that conforms to ``params``, note that it must not contain selector
	:returns: list of what is encoded in the calldata
	"""
	mem = memoryview(data)

	current_off: int = 0
	current_off_0: int = 0

	def with_indirection[T](fn: typing.Callable[[], T], new_self_length) -> T:
		nonlocal current_off, current_off_0

		off = int.from_bytes(mem[current_off : current_off + 32], 'big', signed=False)
		current_off += 32

		old_current_off = current_off
		old_current_off_0 = current_off_0

		# current_off_0 = current_off_0 + self_length + off - 32
		current_off_0 = current_off_0 + off
		# current_off_0 = current_off_0 + off
		current_off = current_off_0

		assert current_off_0 < len(mem)

		res = fn()

		current_off = old_current_off
		current_off_0 = old_current_off_0

		return res

	def read_regular(param: type) -> typing.Any:
		nonlocal current_off, current_off_0
		as_int = _integer_types.get(param, None)
		if as_int is not None:
			res = int.from_bytes(
				mem[current_off : current_off + 32], 'big', signed=as_int.startswith('i')
			)
			current_off += 32
			return res
		elif param is bool:
			res = int.from_bytes(mem[current_off : current_off + 32], 'big', signed=False)
			current_off += 32
			return res != 0
		elif param is Address:
			current_off += 12
			as_bytes = mem[current_off : current_off + 20]
			current_off += 20
			return Address(as_bytes)
		elif param is bytes or param is str:

			def read_bytes_str() -> bytes | str:
				nonlocal current_off
				le = int.from_bytes(mem[current_off : current_off + 32], 'big', signed=False)
				current_off += 32
				as_bytes = mem[current_off : current_off + le]

				if param is bytes:
					return bytes(as_bytes)
				else:
					return str(as_bytes, encoding='utf-8')

			return with_indirection(read_bytes_str, -1)
		elif (origin := typing.get_origin(param)) is not None:
			type_args = typing.get_args(param)

			if origin is tuple:
				if is_dynamic(param):
					return with_indirection(
						lambda: tuple(read_regular(p) for p in type_args), calc_size_here(param)
					)
				else:
					return tuple(read_regular(p) for p in type_args)
			elif origin is list or origin is DynArray:
				[elem] = type_args

				def read_list() -> list:
					nonlocal current_off, current_off_0
					le = int.from_bytes(mem[current_off : current_off + 32], 'big', signed=False)
					current_off_0 += 32
					current_off += 32
					return [read_regular(elem) for _i in range(le)]

				return with_indirection(read_list, -1)
			elif origin is Array:
				assert typing.get_origin(type_args[1]) is typing.Literal
				le = int(*typing.get_args(type_args[1]))
				return [read_regular(type_args[0]) for _i in range(le)]
			else:
				assert False
		else:
			assert False

	return [read_regular(par) for par in params]
