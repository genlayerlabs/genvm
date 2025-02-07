__all__ = ('get_schema',)

import typing
from genlayer.py.types import Address
import types

import collections.abc
import inspect
import dataclasses

import genlayer.py._internal.reflect as reflect


def _is_public(meth) -> bool:
	if meth is None:
		return False
	return getattr(meth, '__public__', False)


def _is_list(t, permissive: bool) -> bool:
	if t is list:
		return True
	if not permissive:
		return False
	try:
		return issubclass(t, collections.abc.Sequence)
	except Exception:
		return False


def _is_dict(t, permissive: bool) -> bool:
	if t is dict:
		return True
	if not permissive:
		return False
	try:
		return issubclass(t, collections.abc.Mapping)
	except Exception:
		return False


def _repr_type(t: typing.Any, permissive: bool) -> typing.Any:
	if t is inspect.Signature.empty:
		return 'any'
	# primitive
	if t is None or t is types.NoneType:
		return 'null'
	if t is bool:
		return 'bool'
	if t is int:
		return 'int'
	if t is str:
		return 'string'
	if t is bytes:
		return 'bytes'
	if t is Address:
		return 'address'
	if _is_list(t, permissive):
		return 'array'
	if _is_dict(t, permissive):
		return 'dict'
	if t is typing.Any:
		return 'any'
	ttype = type(t)
	if ttype is getattr(typing, '_UnionGenericAlias', None) or ttype is types.UnionType:
		return {'$or': [_repr_type(x, permissive) for x in typing.get_args(t)]}
	if dataclasses.is_dataclass(t) and isinstance(t, type):
		try:
			return {
				prop_name: _repr_type(prop_value, permissive)
				for prop_name, prop_value in typing.get_type_hints(t).items()
			}
		except Exception as e:
			raise TypeError(
				'failed to generate dataclass schema',
				{'dataclass': t, **reflect.try_get_lineno(t)},
			) from e
	origin = typing.get_origin(t)
	if origin != None:
		args = typing.get_args(t)
		if _is_dict(origin, permissive):
			assert len(args) == 2
			assert (
				args[0] is str
			), f'dictionary can have only string keys, got {type(args[0])}'
			return {'$dict': _repr_type(args[1], permissive)}
		if origin is tuple:
			if len(args) == 2 and args[1] == ...:
				return [{'$rep': _repr_type(args[0], permissive)}]
			return [_repr_type(a, permissive) for a in args]
		if _is_list(origin, permissive):
			assert len(args) == 1
			return [{'$rep': _repr_type(args[0], permissive)}]
		if origin is typing.Literal:
			return 'any'  # FIXME
	raise TypeError(
		f'type is not supported', {'type': t, 'kind': ttype, **reflect.try_get_lineno(t)}
	)


def _escape_dict_prop(prop: str) -> str:
	if prop.startswith('$'):
		return '$' + prop
	return prop


def _get_params(m: types.FunctionType, *, is_ctor: bool) -> dict:
	import inspect

	try:
		signature = inspect.signature(m)
		params = []
		kwparams = {}

		is_first = True
		for name, par in signature.parameters.items():
			if is_first:
				if name != 'self':
					raise Exception('missing self')
				is_first = False
				continue
			match str(par.kind):
				case 'POSITIONAL_ONLY' | 'POSITIONAL_OR_KEYWORD':
					params.append([name, _repr_type(par.annotation, True)])
				case 'KEYWORD_ONLY':
					kwparams[_escape_dict_prop(name)] = _repr_type(par.annotation, True)
				case kind:
					raise TypeError(
						f'unsupported parameter type {kind} {type(kind)} for `{name}: {par}`'
					)

		ret = {
			'params': params,
			'kwparams': kwparams,
		}
		if not is_ctor:
			ret.update(
				{
					'readonly': getattr(m, '__readonly__', False),
					'ret': _repr_type(signature.return_annotation, True),
				}
			)
		return ret
	except Exception as e:
		raise Exception(
			f"couldn't get schema for method {m}", reflect.try_get_lineno(m)
		) from e


def _get_ctor(contract: type) -> types.FunctionType:
	if not hasattr(contract, '__dict__') or '__init__' not in contract.__dict__:
		raise TypeError('__init__ is absent', contract)
	ctor = getattr(contract, '__init__')
	if not inspect.isfunction(ctor):
		raise TypeError('__init__ is not a function', contract, ctor)
	if _is_public(ctor):
		raise TypeError('__init__ must be private', contract, ctor)
	return ctor


def get_schema(contract: type) -> typing.Any:
	"""
	Uses python type reflections to produce GenVM ABI schema
	"""

	ctor = _get_ctor(contract)

	meths = {
		name: meth
		for name, meth in sorted(inspect.getmembers(contract))
		if inspect.isfunction(meth) and _is_public(meth)
	}

	for k in meths:
		if k.startswith('__'):
			raise TypeError(f'public method names should not start with `__`, `{k}`')

	return {
		'ctor': _get_params(ctor, is_ctor=True),
		'methods': {k: _get_params(v, is_ctor=False) for k, v in meths.items()},
	}
