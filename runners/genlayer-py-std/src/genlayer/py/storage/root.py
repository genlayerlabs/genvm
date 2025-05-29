import typing

from ._internal.generate import InmemManager
from genlayer.py.types import u8, u256, Address
from ._internal.generate import generate_storage, _known_descs
from ._internal.core import ROOT_SLOT_ID, Slot, Manager, InmemManager
from ._internal.core import Indirection, VLA


@generate_storage
class Root:
	"""
	This ABI is known and used by:

	#. genvm
	#. node
	"""

	MANAGER: typing.ClassVar[Manager] = InmemManager()

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

	@staticmethod
	def get() -> 'Root':
		slot = Root.MANAGER.get_store_slot(ROOT_SLOT_ID)
		return _known_descs[Root].get(slot, 0)

	def slot(self) -> Slot:
		return self._storage_slot  # type: ignore

	def get_contract_instance[T](self, typ: typing.Type[T]) -> T:
		slot: Slot = self.slot().indirect(0)
		return _known_descs[typ].get(slot, 0)

	def lock_default(self):
		frozen = self.locked_slots.get()

		frozen.append(self.slot().as_int())
		frozen.append(self.code.slot().as_int())
		frozen.append(self.locked_slots.slot().as_int())
		frozen.append(self.upgraders.slot().as_int())
