__all__ = ('contract_generator',)

import typing
import inspect
from functools import partial

from ..types import Address


class _EthContract[TView, TWrite]:
	__slots__ = ('_view', '_send', 'address')

	def __init__(
		self,
		address: Address,
		view_impl: typing.Callable[['_EthContract'], TView],
		send_impl: typing.Callable[['_EthContract'], TWrite],
	):
		self.address = address
		self._view = view_impl
		self._send = send_impl

	def view(self) -> TView:
		return self._view(self)

	def emit(self) -> TWrite:
		return self._send(self)


class EthContractDeclaration[TView, TWrite](typing.Protocol):
	View: type[TView]
	Write: type[TWrite]


def _generate_methods(
	f_type: typing.Any,
	proxy_name,
	factory: typing.Callable[[str, list, typing.Any], typing.Callable[..., typing.Any]],
) -> typing.Callable[[_EthContract], typing.Any]:
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

		props[name] = factory(name, real_params, ret_annotation)

	def new_init(self, parent):
		self.parent = parent

	props.update(
		{
			'__init__': new_init,
			'__slots__': ('parent',),
		}
	)
	return type(proxy_name, (object,), props)


type _EthGenerator = typing.Callable[[str, list[type], type], typing.Any]


def contract_generator(generate_view: _EthGenerator, generate_send: _EthGenerator):
	def gen[TView, TWrite](
		contr: EthContractDeclaration[TView, TWrite],
	) -> typing.Callable[[Address], _EthContract[TView, TWrite]]:
		view_meths = _generate_methods(
			contr.View, f'{contr.__qualname__}.ViewProxy', factory=generate_view
		)
		send_meths = _generate_methods(
			contr.Write, f'{contr.__qualname__}.WriteProxy', factory=generate_send
		)
		return partial(_EthContract, view_impl=view_meths, send_impl=send_meths)

	return gen
