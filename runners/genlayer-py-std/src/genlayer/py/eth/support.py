__all__ = (
	'InplaceTuple',
	'bytes1',
	'bytes2',
	'bytes3',
	'bytes4',
	'bytes5',
	'bytes6',
	'bytes7',
	'bytes8',
	'bytes9',
	'bytes10',
	'bytes11',
	'bytes12',
	'bytes13',
	'bytes14',
	'bytes15',
	'bytes16',
	'bytes17',
	'bytes18',
	'bytes19',
	'bytes20',
	'bytes21',
	'bytes22',
	'bytes23',
	'bytes24',
	'bytes25',
	'bytes26',
	'bytes27',
	'bytes28',
	'bytes29',
	'bytes30',
	'bytes31',
	'bytes32',
)

import typing


class InplaceTuple:
	# editorconfig-checker-disable
	"""
	This class indicates that tuple should be encoded/decoded in-place.
	Which means that even if it is dynamically sized, it is ignored.
	It is useful for encoding/decoding arguments and returns

	.. code-block:: python
	        tuple[InplaceTuple, str, u256]
	"""

	# editorconfig-checker-enable

	__slots__ = ()


bytes1 = typing.NewType('bytes1', bytes)
bytes2 = typing.NewType('bytes2', bytes)
bytes3 = typing.NewType('bytes3', bytes)
bytes4 = typing.NewType('bytes4', bytes)
bytes5 = typing.NewType('bytes5', bytes)
bytes6 = typing.NewType('bytes6', bytes)
bytes7 = typing.NewType('bytes7', bytes)
bytes8 = typing.NewType('bytes8', bytes)
bytes9 = typing.NewType('bytes9', bytes)
bytes10 = typing.NewType('bytes10', bytes)
bytes11 = typing.NewType('bytes11', bytes)
bytes12 = typing.NewType('bytes12', bytes)
bytes13 = typing.NewType('bytes13', bytes)
bytes14 = typing.NewType('bytes14', bytes)
bytes15 = typing.NewType('bytes15', bytes)
bytes16 = typing.NewType('bytes16', bytes)
bytes17 = typing.NewType('bytes17', bytes)
bytes18 = typing.NewType('bytes18', bytes)
bytes19 = typing.NewType('bytes19', bytes)
bytes20 = typing.NewType('bytes20', bytes)
bytes21 = typing.NewType('bytes21', bytes)
bytes22 = typing.NewType('bytes22', bytes)
bytes23 = typing.NewType('bytes23', bytes)
bytes24 = typing.NewType('bytes24', bytes)
bytes25 = typing.NewType('bytes25', bytes)
bytes26 = typing.NewType('bytes26', bytes)
bytes27 = typing.NewType('bytes27', bytes)
bytes28 = typing.NewType('bytes28', bytes)
bytes29 = typing.NewType('bytes29', bytes)
bytes30 = typing.NewType('bytes30', bytes)
bytes31 = typing.NewType('bytes31', bytes)
bytes32 = typing.NewType('bytes32', bytes)
