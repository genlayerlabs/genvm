import inspect
import typing
import collections.abc
import contextlib

from genlayer.py.types import SizedArray
from genlayer.py.storage import Array


def try_get_lineno(m):
	res = {}
	try:
		res['origin'] = inspect.getsourcefile(m)
	except Exception:
		pass
	try:
		_, lineno = inspect.findsource(m)
		res['line'] = lineno
	except Exception:
		pass
	return res


def repr_generic(origin: typing.Any, args: typing.Iterable[typing.Any]) -> str:
	return f'{repr_type(origin)}[' + ', '.join(map(repr_type, args)) + ']'


def repr_type(t: typing.Any) -> str:
	origin = typing.get_origin(t)
	if origin is not None:
		args = typing.get_args(t)
		return repr_generic(origin, args)
	if isinstance(t, type):
		if hasattr(t, '__qualname__'):
			return t.__qualname__
		if hasattr(t, '__name__'):
			return t.__name__
	return repr(t)


def is_sized_array(
	origin: typing.Any, args: tuple[typing.Any, ...]
) -> tuple[typing.Any, int] | None:
	if origin is not SizedArray and origin is not Array:
		return None
	if len(args) != 2:
		raise TypeError(
			f'expected exactly two type argument, got: {repr_generic(origin, args)}'
		)
	elem_type = args[0]
	size_lit = args[1]
	if typing.get_origin(size_lit) is not typing.Literal:
		raise TypeError(
			f'expected Literal for Array size, got `{size_lit} (in {repr_generic(origin, args)})`'
		)
	size_lit_args = typing.get_args(size_lit)
	if len(size_lit_args) != 1 or type(size_lit_args[0]) is not int:
		raise TypeError(
			f'expected single int Literal for Array size, got `{size_lit_args} (in {repr_generic(origin, args)})`'
		)
	le = size_lit_args[0]
	if le <= 0:
		raise TypeError(f'array size must be strictly positive')
	return (elem_type, le)


def is_array(origin: typing.Any, args: tuple[typing.Any, ...]) -> typing.Any | None:
	if not issubclass(origin, collections.abc.Sequence):
		return None
	if len(args) == 0:
		return typing.Any
	if len(args) != 1:
		raise TypeError(
			f'expected exactly one type argument, got: {repr_generic(origin, args)}'
		)
	return args[0]


def is_tuple(
	origin: typing.Any, args: tuple[typing.Any, ...]
) -> tuple[typing.Any, ...] | None:
	if origin is not tuple and origin is not typing.Tuple:
		return None
	return args


def is_none_type(t: typing.Any) -> bool:
	return t is None or t is type(None)


@contextlib.contextmanager
def context_notes(notes: str) -> typing.Generator[None, None, None]:
	try:
		yield
	except BaseException as e:
		e.add_note(notes)
		raise


@contextlib.contextmanager
def context_field(name: str, type: typing.Any) -> typing.Generator[None, None, None]:
	try:
		yield
	except BaseException as e:
		e.add_note(f'during generating field `{name}: {type!r}`')
		raise


@contextlib.contextmanager
def context_generic_argument(
	origin: typing.Any, args: tuple[typing.Any, ...], type: typing.Any, index: int = -1
) -> typing.Generator[None, None, None]:
	try:
		yield
	except BaseException as e:
		idx = str(index) if index > 0 else '<unknown>'
		e.add_note(
			f'during processing generic argument of `{repr_generic(origin, args)}`, argument `{type!r}`, index `{idx}`'
		)
		raise


@contextlib.contextmanager
def context_type(t: typing.Any) -> typing.Generator[None, None, None]:
	try:
		yield
	except BaseException as e:
		pushed = 'during processing type ' + repr_type(t)
		if len(ln := try_get_lineno(t)) != 0:
			pushed += str(ln)
		e.add_note(pushed)
		raise
