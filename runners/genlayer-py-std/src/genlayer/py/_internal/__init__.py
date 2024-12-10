__all__ = ('create2_address',)

from ..types import Address, u256
from ..keccak import Keccak256


def create2_address(
	contract_address: Address, salt_nonce: u256, chain_id: u256
) -> Address:
	hasher = Keccak256()
	hasher.update(b'\x01')  # CREATE 2 code
	hasher.update(contract_address.as_bytes)
	hasher.update(salt_nonce.to_bytes(32, 'big', signed=False))
	hasher.update(chain_id.to_bytes(32, 'big', signed=False))
	return Address(hasher.digest()[:20])
