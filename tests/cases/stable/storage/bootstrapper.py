# { "Depends": "py-genlayer:test" }

import typing

import genlayer as gl

_CALC_MANAGER = gl.storage.InmemManager()
CODE_SLOT = (
	_CALC_MANAGER.get_store_slot(gl.storage.ROOT_SLOT_ID)
	.indirect(gl.vm.ABI.root_offsets.CODE)
	.id
)
LOCKED_SLOT = (
	_CALC_MANAGER.get_store_slot(gl.storage.ROOT_SLOT_ID)
	.indirect(gl.vm.ABI.root_offsets.LOCKED_SLOTS)
	.id
)

VACANT_SLOT = _CALC_MANAGER.get_store_slot(gl.storage.ROOT_SLOT_ID).indirect(
	gl.vm.ABI.root_offsets.MAJOR
)

SPECIAL_SLOTS = [gl.storage.ROOT_SLOT_ID, CODE_SLOT, LOCKED_SLOT]

SPECIAL_SLOTS_TWINS = {
	slt: VACANT_SLOT.indirect(i).id for i, slt in enumerate(SPECIAL_SLOTS)
}


def get_twin_vla(slot_id: bytes) -> gl.storage.VLA[gl.types.u8]:
	return gl.storage.cast_slot(
		gl.storage.VLA[gl.types.u8], gl.storage.Root.MANAGER, slot_id, 0
	)


def write_to_twin(slot_id: bytes, offset: int, data: bytes):
	as_vla = get_twin_vla(slot_id)
	old_len = len(as_vla)
	as_vla.slot().write(as_vla.data_offset() + offset, data)
	as_vla.set_length(max(old_len, offset + len(data)))


def read_from_twin(slot_id: bytes, offset: int, length: int) -> bytes:
	slot = gl.storage.Root.MANAGER.get_store_slot(slot_id)
	return slot.read(offset + 4, length)


class Contract(gl.contract.Contract):
	def __init__(self, *, save_default_locked_slots: bool = True):
		root = gl.storage.Root.get()

		if save_default_locked_slots:
			twin_slot_id = SPECIAL_SLOTS_TWINS[LOCKED_SLOT]
			locked_slots_dst = gl.storage.cast_slot(
				gl.storage.VLA[gl.types.u256], gl.storage.Root.MANAGER, twin_slot_id, 4
			)
			locked_slots_dst.assign(root.locked_slots.get())

			locked_slots_dst_vla = get_twin_vla(twin_slot_id)
			locked_slots_dst_vla.set_length(
				len(locked_slots_dst) * 8 + locked_slots_dst.data_offset()
			)

		root.locked_slots.get().truncate()

	@gl.public.write
	def write(self, data: list[typing.Any]):
		"""
		Write data to the contract.

		:param data: list of data items to write in format ``(slot: byte[32], offset: u32, data: bytes)``
		"""

		for slot, offset, slice in data:
			if slot in SPECIAL_SLOTS_TWINS.values():
				gl.vm.UserError.immediate('Cannot write to the temporary code slots')
			if twin := SPECIAL_SLOTS_TWINS.get(slot):
				write_to_twin(twin, offset, slice)
			else:
				gl.storage.Root.MANAGER.get_store_slot(slot).write(offset, slice)

	@gl.public.write
	def push_code(self, code: bytes):
		"""
		Push code to the contract. This will write the code to a temporary slot, which can be copied to the code slot in the ``finish`` method.

		:param code: code to push

		.. warning::
			Exactly one of ``write`` or ``push_code`` must be called for code initialization,
			otherwise the contract may be left in undefined state
		"""

		twin_slot_id = SPECIAL_SLOTS_TWINS[CODE_SLOT]
		code_twin_vla = get_twin_vla(twin_slot_id)
		code_vla = gl.storage.cast_slot(
			gl.storage.VLA[gl.types.u8], gl.storage.Root.MANAGER, twin_slot_id, 4
		)
		code_vla.extend(code)
		new_len = len(code_vla)
		code_twin_vla.set_length(new_len + 4)

	@gl.public.write
	def finish(self):
		"""
		Finish bootstrapping. This will copy the code from the temporary slot to the code slot and make it available for execution.
		"""
		BUF_SIZE = 65536
		for dst, src in SPECIAL_SLOTS_TWINS.items():
			vla_from = get_twin_vla(src)
			copy_len = len(vla_from)
			to = gl.storage.Root.MANAGER.get_store_slot(dst)
			for offset in range(0, copy_len, BUF_SIZE):
				chunk_size = min(BUF_SIZE, copy_len - offset)
				data = vla_from.slot().read(vla_from.data_offset() + offset, chunk_size)
				to.write(offset, data)
