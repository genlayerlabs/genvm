from __future__ import annotations

import numpy as np
import typing
from numpy.typing import ArrayLike, DTypeLike
from collections.abc import Sequence


def unwrap(x: ArrayLike | Tensor) -> ArrayLike:
	if isinstance(x, Tensor):
		res = x.compute()
		# x.decref()
		return res
	return x


def shapeOf(x: ArrayLike | Tensor) -> tuple[int, ...] | None:
	if isinstance(x, Tensor):
		return x.shape
	return np.shape(x)


def shapeOfNNull(x: ArrayLike | Tensor) -> tuple[int, ...]:
	if isinstance(x, Tensor):
		assert x.shape is not None
		return x.shape
	return np.shape(x)


def commonShapeBop(x: tuple[int, ...], y: tuple[int, ...]) -> tuple[int, ...]:
	return (np.zeros(x, dtype=np.int8) + np.zeros(y, dtype=np.int8)).shape


def _run_computation(
	computation: typing.Callable[[], np.ndarray],
	exp_shape: tuple[int, ...] | None,
	exp_dtype,
) -> np.ndarray:
	res = computation()
	if not isinstance(res, np.ndarray):
		shp = None
		try:
			shp = np.shape(res)
		except:
			pass
		if shp == ():
			res = np.array(res, dtype=type(res))
	assert isinstance(res, np.ndarray)
	assert (
		exp_shape is None or res.shape == exp_shape
	), f'res {res.shape} exp {exp_shape} (at {computation})'
	assert res.dtype == exp_dtype, f'res {res.dtype} exp {exp_dtype} (at {computation})'
	return res


class Tensor:
	__slots__ = ('_store', '_computation', '_computed', 'shape', 'dtype', '_refs', '_tb')

	_store: 'TensorStorage'
	_computation: typing.Callable[[], np.ndarray]
	_computed: np.ndarray | None
	_refs: int
	shape: tuple[int, ...] | None
	dtype: DTypeLike
	_tb: str | None

	def __repr__(self) -> str:
		return f'{type(self).__name__}[dtype={self.dtype} shape={self.shape}]'

	def __init__(self):
		assert False

	def _populate_tb(self):
		import traceback

		self._tb = ''.join(traceback.format_stack())

	@property
	def ndim(self) -> int:
		assert self.shape is not None
		return len(self.shape)

	def incref(self):
		self._refs += 1

	def decref(self):
		self._refs -= 1
		if self._refs == 0:
			del self._computed
			self._computed = None

	def compute(self) -> np.ndarray:
		if self._computed is None:
			self._computed = _run_computation(self._computation, self.shape, self.dtype)
		return self._computed

	def __neg__(self) -> Tensor:
		return self._store.new(self.shape, self.dtype, lambda: -self.compute())

	def __add__(self, x: ArrayLike | Tensor) -> Tensor:
		return self._store._mk(
			self.dtype,
			lambda: commonShapeBop(shapeOfNNull(self), shapeOfNNull(x)),
			lambda: self.compute() + unwrap(x),
			[self, x],
		)

	def __sub__(self, x: ArrayLike | Tensor) -> Tensor:
		return self._store._mk(
			self.dtype,
			lambda: commonShapeBop(shapeOfNNull(self), shapeOfNNull(x)),
			lambda: self.compute() - unwrap(x),
			[self, x],
		)

	def __mul__(self, x: ArrayLike | Tensor) -> Tensor:
		return self._store._mk(
			self.dtype,
			lambda: commonShapeBop(shapeOfNNull(self), shapeOfNNull(x)),
			lambda: self.compute() * unwrap(x),
			[self, x],
		)

	def __pow__(self, x: ArrayLike | Tensor) -> Tensor:
		single = False
		if shapeOf(x) in ((), (1,)):
			single = True
		if single:
			return self._store._mk(
				self.dtype,
				lambda: shapeOfNNull(self),
				lambda: self.compute() ** unwrap(x),
				[self, x],
			)

		assert False, f'** {x}'  # TODO

	def __truediv__(self, x: ArrayLike | Tensor) -> Tensor:
		return self._store._mk(
			self.dtype,
			lambda: commonShapeBop(shapeOfNNull(self), shapeOfNNull(x)),
			lambda: self.compute() / unwrap(x),
			[self, x],
		)

	def __matmul__(self, x: ArrayLike | Tensor) -> Tensor:
		l_shape = self.shape
		r_shape = shapeOf(x)
		if l_shape is None or r_shape is None:
			new_shape = None
		else:
			n1, n2 = len(l_shape), len(r_shape)
			assert (
				n1 != 0 and n2 != 0
			), (
				f'both arguments to matmul need to be at least 1D, but they are {n1}D and {n2}D'
			)
			assert (
				(L := l_shape[-1]) == (R := r_shape[-min(n2, 2)])
			), (
				f'Input Tensor shapes {l_shape} and {r_shape} cannot be multiplied ({L} != {R})'
			)
			new_shape = l_shape[:-1] + (r_shape[-1],)
		return self._store.new(new_shape, self.dtype, lambda: self.compute() @ unwrap(x))

	# def __and__(self, x) -> Tensor: return self.bitwise_and(x)
	# def __or__(self, x) -> Tensor: return self.bitwise_or(x)
	# def __xor__(self, x) -> Tensor: return self.xor(x)
	# def __lshift__(self, x) -> Tensor: return self.lshift(x)
	# def __rshift__(self, x) -> Tensor: return self.rshift(x)

	def __radd__(self, x: ArrayLike | Tensor) -> Tensor:
		return self._store._mk(
			self.dtype,
			lambda: commonShapeBop(shapeOfNNull(self), shapeOfNNull(x)),
			lambda: unwrap(x) + self.compute(),
			[self, x],
		)

	def __rsub__(self, x: ArrayLike | Tensor) -> Tensor:
		return self._store._mk(
			self.dtype,
			lambda: commonShapeBop(shapeOfNNull(self), shapeOfNNull(x)),
			lambda: unwrap(x) - self.compute(),
			[self, x],
		)

	def __rmul__(self, x: ArrayLike | Tensor) -> Tensor:
		return self._store._mk(
			self.dtype,
			lambda: commonShapeBop(shapeOfNNull(self), shapeOfNNull(x)),
			lambda: unwrap(x) * self.compute(),
			[self, x],
		)

	def __rtruediv__(self, x: ArrayLike | Tensor) -> Tensor:
		return self._store._mk(
			self.dtype,
			lambda: commonShapeBop(shapeOfNNull(self), shapeOfNNull(x)),
			lambda: unwrap(x) / self.compute(),
			[self, x],
		)

	# def __rpow__(self, x: ArrayLike | Tensor) -> Tensor: return self.pow(x, True)
	# def __rtruediv__(self, x) -> Tensor: return self.div(x, True)
	# def __rmatmul__(self, x) -> Tensor: return self.matmul(x, True)
	# def __rand__(self, x) -> Tensor: return self.bitwise_and(x, True)
	# def __ror__(self, x) -> Tensor: return self.bitwise_or(x, True)
	# def __rxor__(self, x) -> Tensor: return self.xor(x, True)

	def __lt__(self, x: ArrayLike | Tensor) -> Tensor:
		return self._store._mk(
			np.bool_,
			lambda: commonShapeBop(shapeOfNNull(self), shapeOfNNull(x)),
			lambda: self.compute() < unwrap(x),
			[self, x],
		)

	def __gt__(self, x: ArrayLike | Tensor) -> Tensor:
		return self._store._mk(
			np.bool_,
			lambda: commonShapeBop(shapeOfNNull(self), shapeOfNNull(x)),
			lambda: self.compute() > unwrap(x),
			[self, x],
		)

	def __ge__(self, x: ArrayLike | Tensor) -> Tensor:
		return self._store._mk(
			np.bool_,
			lambda: commonShapeBop(shapeOfNNull(self), shapeOfNNull(x)),
			lambda: self.compute() >= unwrap(x),
			[self, x],
		)

	def __le__(self, x: ArrayLike | Tensor) -> Tensor:
		return self._store._mk(
			np.bool_,
			lambda: commonShapeBop(shapeOfNNull(self), shapeOfNNull(x)),
			lambda: self.compute() <= unwrap(x),
			[self, x],
		)

	def __ne__(self, x: ArrayLike | Tensor) -> Tensor:
		return self._store._mk(
			np.bool_,
			lambda: commonShapeBop(shapeOfNNull(self), shapeOfNNull(x)),
			lambda: self.compute() != unwrap(x),
			[self, x],
		)

	def __eq__(self, x: ArrayLike | Tensor) -> Tensor:
		return self._store._mk(
			np.bool_,
			lambda: commonShapeBop(shapeOfNNull(self), shapeOfNNull(x)),
			lambda: self.compute() == unwrap(x),
			[self, x],
		)

	def sqrt(self) -> Tensor:
		return self._store._mk(
			self.dtype, lambda: shapeOfNNull(self), lambda: np.sqrt(self.compute()), [self]
		)

	def tanh(self) -> Tensor:
		return self._store._mk(
			self.dtype, lambda: shapeOfNNull(self), lambda: np.tanh(self.compute()), [self]
		)

	def abs(self) -> Tensor:
		return self._store._mk(
			self.dtype, lambda: shapeOfNNull(self), lambda: np.abs(self.compute()), [self]
		)

	def exp(self) -> Tensor:
		return self._store._mk(
			self.dtype, lambda: shapeOfNNull(self), lambda: np.exp(self.compute()), [self]
		)

	def transpose(self, perm: None | Sequence[int]) -> Tensor:
		def shape_compute(perm=perm):
			self_shape = shapeOfNNull(self)
			if perm is None:
				perm = tuple(range(self.ndim)[::-1])
			assert len(perm) == self.ndim
			new_shape = list(range(self.ndim))
			for idx, new_val in enumerate(perm):
				new_shape[idx] = self_shape[new_val]
			return tuple(new_shape)

		return self._store._mk(
			self.dtype, shape_compute, lambda: self.compute().transpose(perm), [self]
		)

	def cast(self, dtype: DTypeLike) -> Tensor:
		if dtype == self.dtype:
			return self
		return self._store._mk(
			dtype, lambda: shapeOfNNull(self), lambda: self.compute().astype(dtype), [self]
		)

	def reshape(self, shape: Sequence[int]) -> Tensor:
		return self._store._mk(
			self.dtype, tuple(shape), lambda: self.compute().reshape(shape), [self]
		)

	def __getitem__(self, idx: tuple[slice | np.ndarray | Tensor, ...]) -> Tensor:
		assert self.ndim == len(idx)
		all_tensors: list[Tensor | ArrayLike] = [self]
		for i in idx:
			if isinstance(i, Tensor):
				all_tensors.append(i)

		# FIXME
		# def get_shape_len(x: slice | np.ndarray | Tensor, mshape: int) -> int:
		# 	if isinstance(x, slice):
		# 		stop = x.stop
		# 		if stop is None:
		# 			stop = mshape
		# 		start = x.start
		# 		if start is None:
		# 			start = 0
		# 		step = x.step
		# 		if step is None:
		# 			step = 1
		# 		return max(0, (stop - start) // step)
		# 	return 1
		# new_shape = tuple(get_shape_len(sl, mshape) for mshape, sl in zip(self.shape, idx))

		pseudo_idx = []

		def shape_computation():
			for x in idx:
				if isinstance(x, Tensor):
					pseudo_idx.append(np.zeros(shapeOfNNull(x), dtype=np.int32))
				else:
					pseudo_idx.append(x)
			return np.shape(np.zeros(shapeOfNNull(self), dtype=np.int8)[tuple(pseudo_idx)])

		def computation():
			def unwrap(i: slice | np.ndarray | Tensor):
				if isinstance(i, Tensor):
					return i.compute()
				return i

			new_idx = tuple(unwrap(i) for i in idx)
			zelf = self.compute()
			rees = zelf[new_idx]
			return rees

		return self._store._mk(self.dtype, shape_computation, computation, all_tensors)

	def shrink(self, arg: tuple[tuple[int, int] | None, ...]) -> Tensor:
		if self.shape is not None:
			return self[
				tuple(
					slice(0, my) if want is None else slice(want[0], want[1])
					for my, want in zip(self.shape, arg)
				)
			]

		def compute():
			zelf = self.compute()
			assert len(arg) == zelf.ndim
			return zelf[
				tuple(
					slice(0, my) if want is None else slice(want[0], want[1])
					for my, want in zip(zelf.shape, arg)
				)
			]

		return self._store.new(
			None,  # FIXME
			self.dtype,
			compute,
		)

	def softmax(self, axis=-1):
		m = self - self.max(axis=axis, keepdim=True)
		e = m.exp()
		ss = e.sum(axis=axis, keepdim=True)
		return e / ss

	def _reduce_calc_new_shape(
		self, axis: None | int | Sequence[int], keepdim: bool
	) -> tuple[int, ...] | None:
		if axis is None:
			return (1,)
		if self.shape is None:
			return None
		if isinstance(axis, int):
			axis = (axis,)
		res: list[int] = []
		for i in range(self.ndim):
			if i in axis or i - len(self.shape) in axis:
				if keepdim:
					res.append(1)
				continue
			res.append(self.shape[i])
		return tuple(res)

	def max(
		self, axis: None | int | Sequence[int] = None, keepdim: bool = False
	) -> Tensor:
		return self._store.new(
			self._reduce_calc_new_shape(axis, keepdim),
			self.dtype,
			lambda: self.compute().max(axis, keepdims=keepdim),
		)

	def min(
		self, axis: None | int | Sequence[int] = None, keepdim: bool = False
	) -> Tensor:
		return self._store.new(
			self._reduce_calc_new_shape(axis, keepdim),
			self.dtype,
			lambda: self.compute().min(axis, keepdims=keepdim),
		)

	def sum(
		self, axis: None | int | Sequence[int] = None, keepdim: bool = False
	) -> Tensor:
		return self._store.new(
			self._reduce_calc_new_shape(axis, keepdim),
			self.dtype,
			lambda: self.compute().sum(axis, keepdims=keepdim),
		)

	def mean(
		self, axis: None | int | Sequence[int] = None, keepdim: bool = False
	) -> Tensor:
		return self._store.new(
			self._reduce_calc_new_shape(axis, keepdim),
			self.dtype,
			lambda: self.compute().mean(axis, keepdims=keepdim),
		)

	def where(self: Tensor, x: Tensor, y: Tensor):
		assert self.dtype == np.bool_
		assert self.shape == x.shape
		assert self.shape == y.shape
		assert x.dtype == y.dtype
		return self._store.new(
			self.shape, x.dtype, lambda: np.where(self.compute(), x.compute(), y.compute())
		)

	def cat(self: Tensor, *args: Tensor | ArrayLike, dim: int = 0) -> Tensor:
		def shape_computation():
			new_shape = list(shapeOfNNull(self))
			for x in args:
				x_shape = shapeOfNNull(x)
				new_shape[dim] += x_shape[dim]
			return tuple(new_shape)

		allTens: list[Tensor | ArrayLike] = [self]
		allTens.extend(args)
		return self._store._mk(
			self.dtype,
			shape_computation,
			lambda: np.concatenate([unwrap(x) for x in allTens], axis=dim),
			allTens,
		)


class ConstTensor(Tensor):
	_computed: np.ndarray

	def incref(self):
		pass

	def decref(self):
		pass


class InputTensor(Tensor):
	def set_input(self, data: np.ndarray):
		assert self.shape is None or np.shape(data) == self.shape
		assert data.dtype == self.dtype
		self._computed = data


class TensorStorage:
	_tensors: list[Tensor]
	_finished: bool

	def __init__(self):
		self._finished = False
		self._tensors = []

	def _mk(
		self,
		dtype: DTypeLike,
		shape_computation: tuple[int, ...] | typing.Callable[[], tuple[int, ...]],
		computation: typing.Callable[[], np.ndarray],
		inputs: list[Tensor | ArrayLike],
	) -> Tensor:
		def is_const(x: Tensor | ArrayLike):
			if isinstance(x, ConstTensor):
				return True
			if isinstance(x, Tensor):
				return False
			return True

		def known_shape(x: Tensor | ArrayLike):
			if isinstance(x, Tensor):
				return x.shape is not None
			return True

		all_shape = all(known_shape(i) for i in inputs)

		if isinstance(shape_computation, tuple):
			shape = shape_computation
		elif all_shape:
			shape = shape_computation()
		else:
			shape = None

		all_const = all(is_const(i) for i in inputs)
		if all_const:
			res = _run_computation(computation, shape, dtype)
			return self.const(res)
		assert not self._finished
		t = Tensor.__new__(Tensor)
		t._tb = None
		t._store = self
		t._computation = computation
		t._computed = None
		t.dtype = dtype
		t.shape = shape
		self._tensors.append(t)

		t._populate_tb()

		return t

	def new(
		self,
		shape: tuple[int, ...] | None,
		dtype: DTypeLike,
		computation: typing.Callable[[], np.ndarray],
	) -> Tensor:
		assert not self._finished
		t = Tensor.__new__(Tensor)
		t._tb = None
		t._store = self
		t._computation = computation
		t._computed = None
		t.dtype = dtype
		t.shape = shape
		self._tensors.append(t)

		t._populate_tb()

		return t

	def input(self, shape: tuple[int, ...] | None, dtype: DTypeLike) -> InputTensor:
		assert not self._finished
		t = InputTensor.__new__(InputTensor)
		t._tb = None
		t._store = self

		def no_computation():
			assert False

		t._computation = no_computation
		t._computed = None
		t.dtype = dtype
		t.shape = shape
		self._tensors.append(t)
		return t

	def const(self, arr: np.ndarray) -> Tensor:
		t = ConstTensor.__new__(ConstTensor)
		t._tb = None
		t._store = self

		def no_computation():
			assert False

		t._computation = no_computation
		t._computed = arr
		t.dtype = arr.dtype
		t.shape = arr.shape
		return t

	def ones(self, shape: tuple[int, ...], dtype: DTypeLike) -> Tensor:
		return self.const(np.ones(shape, dtype))

	def zeros(self, shape: tuple[int, ...], dtype: DTypeLike) -> Tensor:
		return self.const(np.zeros(shape, dtype))

	def finish(self):
		self._finished = True

	def reset(self):
		assert self._finished
		for t in self._tensors:
			t._computed = None
