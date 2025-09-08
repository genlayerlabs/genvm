import functools, io, math
import typing
import operator
from typing import Union, Tuple, Optional, List, Any
from . import DTYPE_MAP_STR
import numpy as np
from collections.abc import Sequence

from .builder import Builder

Tensor = str  # String-based tensor representation


def get_as_const(x: Tensor) -> np.ndarray:
	raise NotImplementedError('get_as_const not available in codegen system')


def prod[T](x: typing.Iterable[T]) -> Union[T, int]:
	return functools.reduce(operator.mul, x, 1)


def flatten[T](l: typing.Iterable[typing.Iterable[T]]):
	return [item for sublist in l for item in sublist]


def _axes(axes, noop_with_empty_axes) -> Sequence[int] | None:
	if axes is not None:
		return axes
	return [] if noop_with_empty_axes else None


def Add(builder: Builder, x: Tensor, other: Tensor, broadcast=None, axis=None):
	return builder.add_decl(f'{x} + {other}')


def Cast(builder: Builder, x: Tensor, to: int, saturate=1):
	typ = DTYPE_MAP_STR[to]
	return builder.add_decl(f'{x}.astype({typ})')


def Concat(builder: Builder, *xs: Tensor, axis):
	tensor_list = f"[{', '.join(xs)}]"
	return builder.add_decl(f'np.concatenate({tensor_list}, axis={axis})')


def Constant(
	builder: Builder,
	value: Tensor | None = None,
	value_float=None,
	value_floats=None,
	value_int=None,
	value_ints=None,
	value_string=None,
	value_strings=None,
):
	if value is not None:
		return value
	np_val: np.ndarray

	if value_float is not None:
		np_val = np.array([value_float], dtype=np.float32)
	elif value_floats is not None:
		np_val = np.array(value_floats, dtype=np.float32)
	elif value_int is not None:
		np_val = np.array([value_int], dtype=np.int64)
	elif value_ints is not None:
		np_val = np.array(value_ints, dtype=np.int64)
	else:
		assert False
	return builder.add_const(np_val)


def Div(builder: Builder, x: Tensor, other: Tensor):
	return builder.add_decl(f'({x} / {other}).astype({x}.dtype)')


def Erf(builder: Builder, x: Tensor):
	return builder.add_decl(f'extra_erf({x})')


def Gather(builder: Builder, x: Tensor, indices: Tensor, axis=0):
	return builder.add_decl(f'np.take({x}, {indices}, axis={axis})')


def Gemm(
	builder: Builder,
	A: Tensor,
	B: Tensor,
	C: Tensor | None = None,
	alpha=1.0,
	beta=1.0,
	transA=0,
	transB=0,
	broadcast=0,
):
	if bool(transA):
		A = builder.add_decl(f'{A}.T')
	if bool(transB):
		B = builder.add_decl(f'{B}.T')
	alph = builder.add_const(alpha)
	ret = builder.add_decl(f'({alph} * ({A} @ {B}))')
	if C is not None:
		bet = builder.add_const(beta)
		ret = builder.add_decl(f'({ret} + {bet} * {C})')
	return ret


def MatMul(builder: Builder, x: Tensor, other: Tensor):
	return builder.add_decl(f'{x} @ {other}')


def Mul(builder: Builder, x: Tensor, other: Tensor):
	return builder.add_decl(f'{x} * {other}')


def Pow(builder: Builder, x: Tensor, other: Tensor):
	return builder.add_decl(f'{x} ** {other}')


def ReduceMean(
	builder: Builder, data: Tensor, axes=None, keepdims=1, noop_with_empty_axes=0
):
	axes_processed = _axes(axes, noop_with_empty_axes)
	if axes_processed is None:
		return builder.add_decl(f'{data}.mean(keepdims={bool(keepdims)})')
	else:
		return builder.add_decl(
			f'{data}.mean(axis=tuple({axes_processed}), keepdims={bool(keepdims)})'
		)


def Reshape(builder: Builder, data: Tensor, shape: Tensor, allowzero=0):
	return builder.add_decl(f'{data}.reshape({shape})')


def Shape(builder: Builder, data: Tensor, end=None, start=0):
	slice_part = f'{start}:{end}' if end is not None else f'{start}:'
	return builder.add_decl(f'np.array({data}.shape[{slice_part}], dtype=np.int64)')


def Softmax_1(builder: Builder, x: Tensor, axis=1):
	x_max = builder.add_decl(f'{x}.max(axis={axis}, keepdims=True)')
	x_shifted = builder.add_decl(f'{x} - {x_max}')
	exp_x = builder.add_decl(f'np.exp({x_shifted})')
	sum_exp = builder.add_decl(f'{exp_x}.sum(axis={axis}, keepdims=True)')
	return builder.add_decl(f'{exp_x} / {sum_exp}')


def Softmax_13(builder: Builder, x: Tensor, axis=-1):
	x_max = builder.add_decl(f'{x}.max(axis={axis}, keepdims=True)')
	x_shifted = builder.add_decl(f'{x} - {x_max}')
	exp_x = builder.add_decl(f'np.exp({x_shifted})')
	sum_exp = builder.add_decl(f'{exp_x}.sum(axis={axis}, keepdims=True)')
	return builder.add_decl(f'{exp_x} / {sum_exp}')


Softmax = {1: Softmax_1, 13: Softmax_13}


def Sqrt(builder: Builder, x: Tensor):
	return builder.add_decl(f'np.sqrt({x})')


def Sub(builder: Builder, x: Tensor, other: Tensor):
	return builder.add_decl(f'{x} - {other}')


def Tanh(builder: Builder, x: Tensor):
	return builder.add_decl(f'np.tanh({x})')


def Transpose(builder: Builder, x: Tensor, perm=None):
	if isinstance(perm, list):
		perm = tuple(perm)
	return builder.add_decl(f'{x}.transpose({perm})')


def Unsqueeze(builder: Builder, data: Tensor, axes):
	return builder.add_decl(f'np.expand_dims({data}, axis=tuple({axes}))')
