__all__ = ('contract_generator', 'ContractDeclaration', 'ContractProxy')

import typing
import inspect
import json
from functools import partial

import genlayer.py._internal.reflect as reflect

from ..types import Address, u256


class TransactionDataKwArgs(typing.TypedDict):
	"""
	Built-in parameters of all transaction messages that a contract can emit

	.. warning::
		parameters are subject to change!
	"""

	value: typing.NotRequired[u256]


def transaction_data_kw_args_serialize(d: dict) -> str:
	if 'value' in d:
		d['value'] = hex(d['value'])
	if 'salt_nonce' in d:
		d['salt_nonce'] = hex(d['salt_nonce'])
	return json.dumps(d, separators=(',', ':'))


class ContractProxy[TView, TWrite]:
	__slots__ = ('_view', '_send', 'address', '_balance', '_transfer')

	def __init__(
		self,
		address: Address,
		view_impl: typing.Callable[['ContractProxy'], TView],
		balance_impl: typing.Callable[['ContractProxy'], u256],
		send_impl: typing.Callable[['ContractProxy', TransactionDataKwArgs], TWrite],
		transfer_impl: typing.Callable[['ContractProxy', TransactionDataKwArgs], None],
	):
		self.address = address
		self._view = view_impl
		self._send = send_impl
		self._balance = balance_impl
		self._transfer = transfer_impl

	def view(self) -> TView:
		return self._view(self)

	def emit(self, **data: typing.Unpack[TransactionDataKwArgs]) -> TWrite:
		return self._send(self, data)

	def emit_transfer(self, **data: typing.Unpack[TransactionDataKwArgs]) -> None:
		self._transfer(self, data)

	@property
	def balance(self) -> u256:
		return self._balance(self)


class ContractDeclaration[TView, TWrite](typing.Protocol):
	"""
	Interface for declaring interfaces of external contracts
	"""

	View: type[TView]
	Write: type[TWrite]


_generate_spec = typing.ParamSpec('_generate_spec')


def _generate_methods(
	f_type: typing.Any,
	proxy_name,
	factory: typing.Callable[[str, tuple, typing.Any], typing.Callable[..., typing.Any]],
) -> typing.Callable[typing.Concatenate[ContractProxy, _generate_spec], typing.Any]:
	props: dict[str, typing.Any] = {}
	for name, val in inspect.getmembers_static(f_type):
		if not inspect.isfunction(val):
			continue
		if name.startswith('__') and name.endswith('__'):
			continue
		sig = inspect.signature(val)

		assert len(sig.parameters) > 0
		assert next(iter(sig.parameters.keys())) == 'self'

		real_params: list = []
		for param_data in list(sig.parameters.values())[1:]:
			assert param_data.kind == inspect.Parameter.POSITIONAL_ONLY
			assert param_data.default is inspect.Parameter.empty
			assert param_data.annotation is not inspect.Parameter.empty

			real_params.append(param_data.annotation)
		ret_annotation = sig.return_annotation
		if ret_annotation is inspect.Parameter.empty:
			ret_annotation = type(None)

		props[name] = factory(name, tuple(real_params), ret_annotation)

	def new_init(
		self, parent, *args: _generate_spec.args, **kwargs: _generate_spec.kwargs
	):
		self._proxy_parent = parent
		self._proxy_args = args
		self._proxy_kwargs = kwargs

	props.update(
		{
			'__init__': new_init,
			'__slots__': ('_proxy_parent', '_proxy_args', '_proxy_kwargs'),
		}
	)
	return type(proxy_name, (object,), props)


type _EthGenerator = typing.Callable[[str, tuple[type], type], typing.Any]


def contract_generator(
	generate_view: _EthGenerator,
	generate_send: _EthGenerator,
	balance_getter: typing.Callable[[ContractProxy], u256],
	transfer: typing.Callable[['ContractProxy', TransactionDataKwArgs], None],
):
	def gen[TView, TWrite](
		contr: ContractDeclaration[TView, TWrite],
	) -> typing.Callable[[Address], ContractProxy[TView, TWrite]]:
		with reflect.context_type(contr):  # type: ignore
			with reflect.context_notes('while generating view methods'):
				view_meths = _generate_methods(
					contr.View, f'{contr.__qualname__}.ViewProxy', factory=generate_view
				)
			with reflect.context_notes('while generating write methods'):
				send_meths: typing.Callable[
					[ContractProxy, TransactionDataKwArgs], typing.Any
				] = _generate_methods(
					contr.Write, f'{contr.__qualname__}.WriteProxy', factory=generate_send
				)
			return partial(
				ContractProxy,
				view_impl=view_meths,
				send_impl=send_meths,
				balance_impl=balance_getter,
				transfer_impl=transfer,
			)

	return gen
