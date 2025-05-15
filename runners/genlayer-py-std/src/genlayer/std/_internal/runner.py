"""
Module that is used to run python contracts in the default way
"""

__all__ = ()

from ..msg import message_raw, MessageRawType
from .result_codes import EntryKind

import genlayer.py.calldata as calldata

import typing
import abc
import dataclasses
from genlayer.py.types import Rollback
import genlayer.py._internal.reflect as reflect

import genlayer.std._internal.gl_call as gl_call


class CalldataSchema(typing.TypedDict, total=False):
	method: str
	args: list[calldata.Decoded]
	kwargs: dict[str, calldata.Decoded]


def _give_result(res_fn: typing.Callable[[], typing.Any]) -> typing.NoReturn:
	try:
		res = res_fn()
	except Rollback as r:
		gl_call.rollback(r.msg)
	gl_call.contract_return(res)


def _handle_main() -> typing.NoReturn:
	from ..genvm_contracts import Contract

	import genlayer.py.get_schema as _get_schema

	@dataclasses.dataclass
	class MethodResolverInfo:
		cd: CalldataSchema
		msg: MessageRawType
		contract_type: type[Contract]

	def get_contract_instance(contract_type: type[Contract]) -> Contract:
		from .storage import STORAGE_MAN, ROOT_STORAGE_ADDRESS

		top_slot = STORAGE_MAN.get_store_slot(ROOT_STORAGE_ADDRESS)
		from ...py.storage._internal.generate import _known_descs

		return _known_descs[contract_type].get(top_slot, 0)

	def check_abstracts(ctx: MethodResolverInfo, meth: typing.Callable) -> str | None:
		if not _get_schema._is_public(meth):
			return f'call to private method `{meth}`'
		if getattr(meth, '__isabstractmethod__', False):
			return f'method is abstract `{meth}`'
		if ctx.msg['value'] > 0 and not getattr(meth, _get_schema.PAYABLE_ATTR, False):
			return f'called non-payable method `{meth} with non-zero value`'
		return None

	def resolve_method(ctx) -> typing.Callable:
		if ctx.msg['is_init']:
			meth = getattr(__known_contact__, '__init__')
			if _get_schema._is_public(meth):
				raise TypeError(f'__init__ must be private')
			if meth is object.__init__:
				raise TypeError('improper contract: define __init__')

			return meth
		# now it is not init
		match ctx.cd.get('method', ''):
			case '#error':
				# no checks
				return ctx.contract_type.__on_errored_message__
			case '#get-schema':
				_give_result(ctx.contract_type.__get_schema__)
			case '':
				if err := check_abstracts(ctx, ctx.contract_type.__receive__):
					if err2 := check_abstracts(
						ctx, ctx.contract_type.__handle_undefined_method__
					):
						exc = ValueError(err2)
						exc.add_note(err)
						raise exc
					else:
						contract = get_contract_instance(ctx.contract_type)
						_give_result(
							lambda: contract.__handle_undefined_method__(
								'', ctx.cd.get('args', []), ctx.cd.get('kwargs', {})
							)
						)
				else:
					return ctx.contract_type.__receive__
			case x:
				if x.startswith('__'):
					raise ValueError('calls to methods that start with __ is forbidden')
				if x.startswith('#'):
					raise ValueError(f'unknown special method {x}')
				meth = getattr(ctx.contract_type, x, None)
				if meth is not None:
					if err := check_abstracts(ctx, meth):
						raise ValueError(err)
					return meth
				if err := check_abstracts(ctx, ctx.contract_type.__handle_undefined_method__):
					raise ValueError(err)
				contract = get_contract_instance(ctx.contract_type)
				_give_result(
					lambda: contract.__handle_undefined_method__(
						ctx.cd.get('method', ''), ctx.cd.get('args', []), ctx.cd.get('kwargs', {})
					)
				)

	# load contract, it should set __known_contact__
	import contract as _user_contract_module  # type: ignore

	from ..genvm_contracts import __known_contact__

	if __known_contact__ is None:
		raise Exception('no contract defined')

	cd_raw = calldata.decode(message_raw['entry_data'])
	if not isinstance(cd_raw, dict):
		raise TypeError(
			f'invalid calldata, expected dict got `{reflect.repr_type(cd_raw)}`'
		)

	cd = typing.cast(CalldataSchema, cd_raw)
	ctx = MethodResolverInfo(cd, message_raw, __known_contact__)
	meth2call = resolve_method(ctx)

	contract_instance = get_contract_instance(__known_contact__)
	_give_result(
		lambda: meth2call(contract_instance, *cd.get('args', []), **cd.get('kwargs', {}))
	)


match message_raw['entry_kind']:
	case EntryKind.MAIN:
		_handle_main()
	case EntryKind.SANDBOX:
		import cloudpickle

		runner = cloudpickle.loads(message_raw['entry_data'])

		_give_result(runner)
	case EntryKind.CONSENSUS_STAGE:
		import cloudpickle

		runner = cloudpickle.loads(message_raw['entry_data'])
		stage_data = message_raw['entry_stage_data']

		_give_result(lambda: runner(stage_data))
	case x:
		raise ValueError(f'invalid entry kind {x}')
