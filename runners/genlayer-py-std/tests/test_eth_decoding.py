import pytest

import genlayer.py.eth as genvm_eth

from genlayer.py.types import *
from genlayer.py.storage import *


@pytest.mark.parametrize(
	'typ,data',
	[
		(u32, [8, 0]),
		(u256, [18, 0]),
		(i32, [-32, 0, 32]),
		(Address, [Address(b'\x30' * 20)]),
		(bool, [True, False]),
		(str, ['', 'a123', 'looong string' * 30, 'руские буквы']),
		(bytes, [b'', b'a123', b'looong string' * 30]),
		(tuple[u32, u32], [(1, 2)]),
		(tuple[u32, str, u32], [(1, 'str!', 2)]),
		(list[u32], [[], [1], [2, 3, 4, 5]]),
		(list[str], [[], ['1'], ['yet another long string' * 20] * 15]),
		(list[list[str]], [[[]], [['1']], [['1', '2'], ['3', '4']]]),
		(Array[Array[u8, typing.Literal[3]], typing.Literal[2]], [[[1, 2, 3], [4, 5, 6]]]),
	],
)
def test_eth_decoding_for_type(typ: type, data: list[typing.Any]):
	meth = genvm_eth.MethodEncoder('test', [typ], type(None))
	for d in data:
		print(f'============ {typ} {d}')
		encoded = meth.encode([d])[4:]
		for i in range(0, len(encoded), 32):
			print(f'{i}\t{encoded[i:i+32].hex()}')
		assert [d] == genvm_eth.decode([typ], encoded)


@pytest.mark.parametrize(
	'params,data',
	[
		([u32, u256, i32, Address], [8, 18, 32, Address(b'\x30' * 20)]),
		([str, str], ['first', 'second']),
		([str, str], ['first' * 29, 'second' * 29]),
		([str, u32, bytes], ['first' * 29, 18, b'second' * 29]),
		(
			[list[list[u256]], list[Address]],
			[
				[[123, 456], [789]],
				[
					Address('0x5B38Da6a701c568545dCfcB03FcB875f56beddC4'),
					Address('0x7b38da6a701c568545dcfcb03fcb875f56bedfb3'),
				],
			],
		),
		(
			[list[tuple[str, Address, str]]],
			[
				[
					('str1', Address('0x5B38Da6a701c568545dCfcB03FcB875f56beddC4'), 'str2'),
					(
						'str3',
						Address('0x7b38da6a701c568545dcfcb03fcb875f56bedfb3'),
						'my last string' * 20,
					),
				],
			],
		),
	],
)
def test_eth_decoding_inverse(params: list[type], data: list[typing.Any]):
	assert len(params) == len(data)
	meth = genvm_eth.MethodEncoder('test', params, type(None))
	encoded = meth.encode(data)[4:]
	assert data == genvm_eth.decode(params, encoded)
