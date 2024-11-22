__all__ = ('eth_contract_generator',)

import typing
import inspect
from functools import partial

from ..types import Address


class _EthViewMethod:
	__slots__ = ('parent',)

	def __init__(self, parent: '_EthContract'):
		self.parent = parent

	def __call__(self, *args) -> None:
		assert False


class _EthContract[TView, TSend]:
	__slots__ = ('_view', '_send', 'address')

	def __init__(
		self,
		address: Address,
		view_impl: typing.Callable[['_EthContract'], TView],
		send_impl: typing.Callable[['_EthContract'], TSend],
	):
		self.address = address
		self._view = view_impl
		self._send = send_impl

	def view(self) -> TView:
		return self._view(self)

	def send(self) -> TSend:
		return self._send(self)


class EthContractDeclaration[TView, TSend](typing.Protocol):
	View: type[TView]
	Send: type[TSend]


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
		ret_annot = sig.return_annotation
		if ret_annot is inspect.Parameter.empty:
			ret_annot = type(None)

		props[name] = factory(name, real_params, ret_annot)

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


def eth_contract_generator(generate_view: _EthGenerator, generate_send: _EthGenerator):
	def gen[TView, TSend](
		contr: EthContractDeclaration[TView, TSend],
	) -> typing.Callable[[Address], _EthContract[TView, TSend]]:
		view_meths = _generate_methods(
			contr.View, f'{contr.__qualname__}.ViewProxy', factory=generate_view
		)
		send_meths = _generate_methods(
			contr.Send, f'{contr.__qualname__}.SendProxy', factory=generate_send
		)
		return partial(_EthContract, view_impl=view_meths, send_impl=send_meths)

	return gen
