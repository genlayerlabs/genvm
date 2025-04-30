import typing
import os

if typing.TYPE_CHECKING or os.getenv('GENERATING_DOCS', 'false') == 'true':
	import collections.abc

	type _Fd = int
	type _FdErroring = int

	### fast methods ###
	def storage_read(slot: bytes, off: int, len: int) -> bytes: ...
	def storage_write(slot: bytes, off: int, what: collections.abc.Buffer) -> bytes: ...

	def get_balance(address: bytes) -> int: ...
	def get_self_balance() -> int: ...

	### gl_call ##
	def gl_call(data: bytes) -> _Fd: ...
else:
	from _genlayer_wasi import *
