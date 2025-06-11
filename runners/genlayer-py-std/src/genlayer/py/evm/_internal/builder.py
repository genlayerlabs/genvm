from .type_dicts import *
from .codecs import ArrayCodec, DynArrayCodec, TupleCodec, Codec
from ..support import InplaceTuple
import genlayer.py._internal.reflect as reflect


def build[T](typ: typing.Type[T]) -> Codec[T]:
	assert not isinstance(typ, tuple)

	if (val := primitive_types_dict.get(typ)) is not None:
		return val

	with reflect.context_type(typ):
		origin = typing.get_origin(typ)
		if origin is None:
			raise TypeError(f'unsupported type `{typ}`')

		args = typing.get_args(typ)

		if (tup := reflect.is_tuple(origin, args)) is not None:
			if tup[0] is InplaceTuple:
				return TupleCodec(tuple(build(e) for e in tup[1:]), True)  # type: ignore
			else:
				return TupleCodec(tuple(build(e) for e in tup), False)  # type: ignore

		if (elem := reflect.is_sized_array(origin, args)) is not None:
			return ArrayCodec(build(elem[0]), elem[1])  # type: ignore

		if (elem := reflect.is_array(origin, args)) is not None:
			return DynArrayCodec(build(elem))  # type: ignore

		raise TypeError(f'unknown type `{typ}`')
