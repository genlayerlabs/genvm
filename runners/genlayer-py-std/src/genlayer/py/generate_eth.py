__all__ = ('eth_contract',)
from .keccak import Keccak256


from genlayer.py.types import *
from genlayer.py.storage import *
from collections import deque
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
	simp = _simple.get(t, None)
	if simp is not None:
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
	assert False


class EthMethod:
	name: str
	params: list[type]
	ret: type
	selector: bytes

	def __init__(self, name: str, params: list[type], ret: type):
		self.name = name
		self.params = params
		self.ret = ret
		sig = self.make_sig()
		self.selector = Keccak256(sig.encode('utf-8')).digest()[:4]

	def make_sig(self) -> str:
		sig: list[str] = [self.name, '(']
		for i, par in enumerate(self.params):
			if i != 0:
				sig.append(',')
			sig.append(get_type_eth_name(par))
		sig.append(')')
		return ''.join(sig)

	def encode(self, args: list[typing.Any]) -> bytes:
		assert len(args) == len(self.params)

		result: bytearray = bytearray()
		result.extend(self.selector)

		queue: deque[typing.Callable[[], None]] = deque()

		def put_offset_at(off: int) -> None:
			to_put = len(result) - len(self.selector)
			memoryview(result)[off : off + 32] = int.to_bytes(to_put, 32, 'big')

		def put_iloc():
			off = len(result)
			result.extend(b'\x00' * 32)
			queue.append(partial(put_offset_at, off))

		def put_regular(param: type, arg: typing.Any) -> None:
			as_int = _integer_types.get(param, None)
			if as_int is not None:
				result.extend(int.to_bytes(arg, 32, 'big', signed=as_int.startswith('u')))
			elif param is bool:
				result.extend(int.to_bytes(1 if arg else 0, 32, 'big'))
			elif param is Address:
				result.extend(b'\x00' * 12)
				result.extend(arg.as_bytes)
			elif param is bytes:
				put_iloc()
				as_bytes = typing.cast(bytes, arg)

				def put_bytes():
					result.extend(int.to_bytes(len(as_bytes), 32, 'big'))
					result.extend(as_bytes)
					result.extend(b'\x00' * ((32 - len(as_bytes) % 32) % 32))

				queue.append(put_bytes)
			elif param is str:
				put_iloc()
				as_bytes = typing.cast(str, arg).encode('utf-8')

				def put_str():
					result.extend(int.to_bytes(len(as_bytes), 32, 'big'))
					result.extend(as_bytes)
					result.extend(b'\x00' * ((32 - len(as_bytes) % 32) % 32))

				queue.append(put_str)
			elif (origin := typing.get_origin(param)) is not None:
				type_args = typing.get_args(param)
				if origin is Array or origin is list:
					put_iloc()
					as_seq = typing.cast(collections.abc.Sequence, arg)

					def put_arr():
						result.extend(int.to_bytes(len(as_seq), 32, 'big'))
						for i in range(len(as_seq)):
							queue.append(partial(put_regular, type_args[0], as_seq[i]))

					queue.append(put_arr)
				elif origin is tuple:
					# FIXME(kp2pml30) this is likely incorrect for dynamic types
					for p, a in zip(type_args, arg):
						put_regular(p, a)
				else:
					assert False
			else:
				assert False

		queue.extend(partial(put_regular, p, a) for p, a in zip(self.params, args))

		while len(queue) > 0:
			queue.popleft()()

		return bytes(result)


def eth_contract(cls):
	return cls
