__all__ = ('Root',)

import typing

from ._internal.generate import InmemManager
from genlayer.types import u8, u256, Address
from ._internal.generate import generate_storage, _known_descs
from genlayer.storage.core import ROOT_SLOT_ID, Slot, Manager, InmemManager
from genlayer.storage.core import Indirection, VLA
from genlayer.vm import public_abi


@generate_storage
class Root:
	"""
	This ABI is known and used by:

	#. genvm
	#. node
	"""

	MANAGER: typing.ClassVar[Manager] = InmemManager()
	"""
	Manager instance for the storage.

	It is set to an actual storage manager in the runtime, but can be overridden for testing purposes.
	"""

	major: u8
	"""
	Major version of the GenVM contract expects
	"""

	contract_instance: Indirection[None]

	code: Indirection[VLA[u8]]
	"""
	contract code
	"""
	locked_slots: Indirection[VLA[u256]]
	"""
	Slot ids that can not be modified after deployment. Use :py:func:`Slot.as_int` for conversion of Slot to :py:class:`int`
	By default it will be populated by ``code``, ``frozen_slots``
	"""
	upgraders: Indirection[VLA[Address]]

	code_slot: u256
	"""
	Slot id the contract code is stored at, as a raw 32-byte value. If zero (the
	default), the code is read from the default ``code`` slot; otherwise it is read
	from the pointed-to slot (see ``chain:`` runner ids).
	"""

	@staticmethod
	def get() -> 'Root':
		"""
		Return the root storage instance.

		:returns: singleton root object
		"""
		slot = Root.MANAGER.get_store_slot(ROOT_SLOT_ID)
		return _known_descs[Root].get(slot, 0)

	def slot(self) -> Slot:
		"""
		Return the storage slot backing this root.

		:returns: underlying storage slot
		"""
		return self._storage_slot  # type: ignore

	def get_vacant_slot(self) -> Slot:
		"""
		This slot can be used to store data without worrying about overwriting contract data.
		Useful for bootstrapping contract storage
		"""
		return self.slot().indirect(public_abi.root_offsets.CODE_SLOT)

	def get_contract_instance[T](self, typ: typing.Type[T], /) -> T:
		"""
		Return the contract instance deserialized as the given type.

		:param typ: storage-allowed type to deserialize into
		:returns: contract instance
		"""
		slot: Slot = self.slot().indirect(public_abi.root_offsets.CONTRACT)
		return _known_descs[typ].get(slot, 0)

	def lock_default(self):
		"""
		Lock the default set of slots (root, code, locked_slots, upgraders) to prevent modification after deployment.
		"""
		frozen = self.locked_slots.get()

		frozen.append(self.slot().as_int())
		frozen.append(self.code.slot().as_int())
		frozen.append(self.locked_slots.slot().as_int())
		frozen.append(self.upgraders.slot().as_int())
