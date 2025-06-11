"""
This module provides some "advanced" features that can be used for optimizations

.. warning::
	If you are using something "advanced" you must know what you do
"""

__all__ = (
	'user_error_immediate',
	'emit_raw_event',
)

import typing

import genlayer.py.calldata as calldata
import collections.abc

import genlayer.gl._internal.gl_call as gl_call


def emit_raw_event(
	name: str,
	indexed_fields_names: collections.abc.Sequence[str],
	blob: calldata.Encodable,
) -> None:
	"""
	Emits a raw event with the given name, indexed fields and blob of data.
	"""
	gl_call.gl_call_generic(
		{
			'EmitEvent': {
				'name': name,
				'indexed_fields': indexed_fields_names,
				'blob': blob,
			}
		},
		lambda _x: None,
	).get()


def user_error_immediate(reason: str) -> typing.NoReturn:
	"""
	Performs an immediate error, current VM won't be able to handle it, stack unwind will not happen
	"""

	gl_call.rollback(reason)
