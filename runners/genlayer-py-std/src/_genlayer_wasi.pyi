__all__ = (
	'storage_read',
	'storage_write',
	'get_balance',
	'get_self_balance',
	'gl_call',
)

import collections.abc

type _Fd = int
type _FdErroring = int

### fast methods ###
def storage_read(slot: bytes, off: int, buf: collections.abc.Buffer, /) -> None: ...
def storage_write(slot: bytes, off: int, what: collections.abc.Buffer, /) -> None: ...
def get_balance(address: bytes, /) -> int: ...
def get_self_balance() -> int: ...

### gl_call ##
def gl_call(data: bytes, /) -> _Fd: ...
