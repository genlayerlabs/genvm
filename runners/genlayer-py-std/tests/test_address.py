from genlayer import Address

import pytest


@pytest.mark.parametrize(
	'as_str',
	[
		'0x03FB09251eC05ee9Ca36c98644070B89111D4b3F',
		'0x90F8bf6A479f320ead074411a4B0e7944Ea8c9C1',
	],
)
def test_addr(as_str: str):
	addr = Address(as_str.lower())
	assert addr.as_hex == as_str
