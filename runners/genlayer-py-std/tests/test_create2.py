from genlayer.py._internal import create2_address
from genlayer.py.types import Address, u256


def test_create2_bytes():
	assert create2_address(
		Address('0x03FB09251eC05ee9Ca36c98644070B89111D4b3F'), u256(127), u256(255)
	) == Address('0x31a38fac42349DC16a84A22FbBACCBb6E238B7F9')
