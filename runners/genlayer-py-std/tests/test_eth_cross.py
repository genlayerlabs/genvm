import eth_abi.abi as eth
import genlayer.py.evm as genvm_eth
from genlayer.py.types import *

import pytest


def test_bytes3():
	assert eth.encode(['bytes3'], [b'123']) == genvm_eth.encode(
		genvm_eth.bytes3, genvm_eth.bytes3(b'123')
	)


@pytest.mark.parametrize('val', [True, False])
def test_bool(val: bool):
	assert eth.encode(['bool'], [val]) == genvm_eth.encode(bool, val)


@pytest.mark.parametrize('val', [-1, -5, 2**31 - 1, -(2**31)])
def test_i32(val: int):
	assert eth.encode(['int32'], [val]) == genvm_eth.encode(i32, i32(val))


@pytest.mark.parametrize(
	'val',
	[
		[],
		[([], []), ([], [])],
		[([], [('1', '5')]), ([], [])],
		[([], []), ([('2', '3')], [])],
		[([('1', '2')], [('3', '4')]), ([('6', '7'), ('8', '9')], [('0000', '1111')])],
		[
			([('1', '2')], [('3', '4')]),
			([('6', '7'), ('8', '9')], [('0000' * 64, '1111' * 64)] * 40),
		],
	],
)
def test_list_tuple_list_tuple_str(
	val: list[tuple[list[tuple[str, str]], list[tuple[str, str]]]],
):
	typ = list[tuple[list[tuple[str, str]], list[tuple[str, str]]]]
	enc = genvm_eth.MethodEncoder('', (typ,), type(None))
	assert eth.encode([enc._encoder.name[1:-1]], [val]) == enc.encode_call((val,))[4:]

	assert genvm_eth.decode(typ, genvm_eth.encode(typ, val)) == val


@pytest.mark.parametrize(
	'val',
	[
		['', '', ''],
		['1', '2', '3'],
		['1', '2' * 128, '3'],
		['1' * 128, '2', '3'],
	],
)
def test_sized_array(val: list[str]):
	typ = SizedArray[str, typing.Literal[3]]
	exp = eth.encode(['string[3]'], [val])
	got = genvm_eth.encode(typ, val)

	assert exp == got

	assert genvm_eth.decode(typ, genvm_eth.encode(typ, val)) == val


@pytest.mark.parametrize(
	'val',
	[
		(('', ''), ('', '')),
		(('a', 'b'), ('c', 'd')),
		(('a' * 100, 'b'), ('c', 'd' * 100)),
	],
)
def test_tuple_tuple_str(val: tuple[tuple[str, str], tuple[str, str]]):
	typ = tuple[tuple[str, str], tuple[str, str]]
	enc = genvm_eth.MethodEncoder('', (typ,), type(None))
	assert eth.encode([enc._encoder.name[1:-1]], [val]) == enc.encode_call((val,))[4:]

	assert genvm_eth.decode(typ, genvm_eth.encode(typ, val)) == val


@pytest.mark.parametrize(
	'val',
	[
		(['', ''], ['', '', '']),
		(['a', 'b'], ['c', 'd', 'e']),
		(['a' * 100, 'b'], ['c', 'd' * 100, 'e']),
	],
)
def test_tuple_sized_str(
	val: tuple[SizedArray[str, typing.Literal[2]], SizedArray[str, typing.Literal[3]]],
):
	typ = tuple[SizedArray[str, typing.Literal[2]], SizedArray[str, typing.Literal[3]]]
	enc = genvm_eth.MethodEncoder(
		'',
		(typ,),
		type(None),
	)
	assert eth.encode([enc._encoder.name[1:-1]], [val]) == enc.encode_call((val,))[4:]

	assert genvm_eth.decode(typ, genvm_eth.encode(typ, val)) == val
