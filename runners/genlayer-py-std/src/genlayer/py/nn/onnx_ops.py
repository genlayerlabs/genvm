import functools, io, math
import typing
import operator
from typing import Union, Tuple, Optional, List, Any
from . import DTYPE_MAP
import numpy as np
from collections.abc import Sequence

from .tensor import Tensor, TensorStorage, ConstTensor, shapeOfNNull


def get_as_const(x: Tensor) -> np.ndarray:
	assert isinstance(x, ConstTensor), f'not const {x} {x._tb}'
	return x.compute()


def prod[T](x: typing.Iterable[T]) -> Union[T, int]:
	return functools.reduce(operator.mul, x, 1)


def flatten[T](l: typing.Iterable[typing.Iterable[T]]):
	return [item for sublist in l for item in sublist]


# FIXME
# it looks suspicious...
def _axes(axes, noop_with_empty_axes) -> Sequence[int] | None:
	if axes is not None and not (isinstance(axes, Tensor) and axes.shape == (0,)):
		return axes
	return [] if noop_with_empty_axes else None


def _pad_left(*shapes: Tuple[int, ...]) -> Tuple[Tuple[int, ...], ...]:
	max_dim = max(len(shape) for shape in shapes)
	return tuple((1,) * (max_dim - len(shape)) + shape for shape in shapes)


def _broadcast_shape(*shapes: Tuple[int, ...]) -> Tuple[int, ...]:
	return tuple(
		0 if 0 in nth_dim_sizes else max(nth_dim_sizes)
		for nth_dim_sizes in zip(*_pad_left(*shapes))
	)


def Add(x: Tensor, other: Tensor, broadcast=None, axis=None):
	#  if x.dtype == dtypes.float else (x + other).cast(x.dtype)
	return x + other


def Cast(x: Tensor, to: int, saturate=1):
	return x.cast(DTYPE_MAP[to])


def Concat(*xs: Tensor, axis):
	return Tensor.cat(*xs, dim=axis)


def Constant(
	value: Tensor | None = None,
	value_float=None,
	value_floats=None,
	value_int=None,
	value_ints=None,
	value_string=None,
	value_strings=None,
	*,
	storage: TensorStorage,
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
	return storage.const(np_val)


def Div(x: Tensor, other: Tensor):
	return (x / other).cast(x.dtype)


def Erf(x: Tensor):
	t = 1.0 / (1.0 + 0.3275911 * x.abs())
	term1 = 0.254829592 * t
	term2 = -0.284496736 * t**2
	term3 = 1.421413741 * t**3
	term4 = -1.453152027 * t**4
	term5 = 1.061405429 * t**5
	y = term1 + term2 + term3 + term4 + term5
	z = 1.0 - y * (-x * x).exp()
	return (x > 0).where(z, -z)


def Gather(x: Tensor, indices: Tensor, axis=0):
	if x.shape is not None:
		return x[tuple(slice(None) if i != axis else indices for i in range(x.ndim))]
	else:
		xx = x
		iindices = indices

		def compute():
			x = xx.compute()
			indices = iindices.compute()
			return x[tuple(slice(None) if i != axis else indices for i in range(x.ndim))]

		return x._store.new(None, x.dtype, compute)


def Gemm(
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
		A = A.transpose(None)
	if bool(transB):
		B = B.transpose(None)
	ret = alpha * (A @ B)
	if C is not None:
		if broadcast:
			if ret.shape is None or C.shape is None:

				def compute():
					c = C.compute()
					new_shape = [-1 if i < len(c.shape) else 1 for i in range(ret.compute().ndim)]
					return c.reshape(new_shape[::-1])

				C_new = C._store.new(None, C.dtype, compute)
			else:
				new_shape = [-1 if i < len(C.shape) else 1 for i in range(ret.ndim)]
				C_new = C.reshape(new_shape[::-1])
		else:
			C_new = C
		ret = ret + beta * C_new
	return ret


def MatMul(x: Tensor, other: Tensor):
	return x @ other


def Mul(x: Tensor, other: Tensor):
	return x * other


def Pow(x: Tensor, other: Tensor):
	return x**other


def ReduceMean(data: Tensor, axes=None, keepdims=1, noop_with_empty_axes=0):
	return data.mean(_axes(axes, noop_with_empty_axes), keepdim=bool(keepdims))


def Reshape(data: Tensor, shape: Tensor, allowzero=0):
	if isinstance(shape, ConstTensor):
		new_shape = shape.shape
	else:
		new_shape = None

	def compute():
		dat = data.compute()
		return dat.reshape(
			tuple(
				int(x) if x != 0 else (0 if allowzero else dat)
				for i, x in enumerate(shape.compute())
			)
		)

	return data._store.new(new_shape, data.dtype, compute)
	return data.reshape(
		tuple(
			int(x) if x != 0 else (0 if allowzero else data.shape[i])
			for i, x in enumerate(get_as_const(shape))
		)
	)


def Shape(data: Tensor, end=None, start=0, *, storage: TensorStorage):
	if data.shape is not None:
		return storage.const(np.array(data.shape[start:end], dtype=np.int64))
	else:
		return storage.new(
			None, np.int64, lambda: np.array(data.compute().shape[start:end], dtype=np.int64)
		)


def Softmax_1(x: Tensor, axis=1):
	return x.softmax(axis)


def Softmax_13(x: Tensor, axis=-1):
	return x.softmax(axis)


Softmax = {1: Softmax_1, 13: Softmax_13}


def Sqrt(x: Tensor):
	return x.sqrt()


def Sub(x: Tensor, other: Tensor):
	return x - other


def Tanh(x: Tensor):
	return x.tanh()


def Transpose(x: Tensor, perm=None):
	if isinstance(perm, list):
		perm = tuple(perm)
	return x.transpose(perm)


def Unsqueeze(data: Tensor, axes):
	axes = get_as_const(axes)
	axes = [x + data.ndim if x < 0 else x for x in axes]
	axes.sort()

	def shape_compute():
		new_shape = list(shapeOfNNull(data))
		for axis in axes:
			new_shape.insert(axis, 1)
		return tuple(new_shape)

	def compute():
		dat = data.compute()
		new_shape = list(dat.shape)
		for axis in axes:
			new_shape.insert(axis, 1)
		return dat.reshape(tuple(new_shape))

	return data._store._mk(data.dtype, shape_compute, compute, [data])
