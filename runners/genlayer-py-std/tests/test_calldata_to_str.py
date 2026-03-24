import pytest

from genlayer.calldata import to_str, CalldataEncodable, DecodingError
from genlayer.types import Address


def test_none():
	assert to_str(None) == 'null'


def test_true():
	assert to_str(True) == 'true'


def test_false():
	assert to_str(False) == 'false'


def test_string():
	assert to_str('hello') == '"hello"'


def test_string_with_quotes():
	assert to_str('say "hi"') == '"say \\"hi\\""'


def test_int():
	assert to_str(42) == '42'


def test_negative_int():
	assert to_str(-1) == '-1'


def test_bytes():
	assert to_str(b'\xab\xcd') == 'b#abcd'


def test_bytearray():
	assert to_str(bytearray(b'\x01\x02')) == 'b#0102'


def test_memoryview():
	assert to_str(memoryview(b'\xff')) == 'b#ff'


def test_address():
	addr = Address(b'\x01' * 20)
	assert to_str(addr) == 'addr#' + '01' * 20


def test_list():
	assert to_str([1, 2, 3]) == '[1,2,3]'


def test_empty_list():
	assert to_str([]) == '[]'


def test_nested_list():
	assert to_str([[1], [2, 3]]) == '[[1],[2,3]]'


def test_dict():
	assert to_str({'a': 1}) == '{"a":1}'


def test_empty_dict():
	assert to_str({}) == '{}'


def test_dict_multiple_keys():
	result = to_str({'x': True, 'y': None})
	assert result == '{"x":true,"y":null}'


def test_nested():
	assert to_str({'items': [1, 'two', None]}) == '{"items":[1,"two",null]}'


def test_calldata_encodable():
	class Custom(CalldataEncodable):
		def __to_calldata__(self):
			return [1, 2]

	assert to_str(Custom()) == '[1,2]'


def test_unencodable():
	with pytest.raises(DecodingError):
		to_str(3.14)
