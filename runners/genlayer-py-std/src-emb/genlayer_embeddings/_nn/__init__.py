"""
Basic support for generating embeddings

This file is highly inspired by `tinygrad <https://github.com/tinygrad/tinygrad>`_ (MIT license)
"""

from __future__ import annotations

__all__ = ('get_run_onnx', 'Tensor', 'TensorStorage')


from .tensor import Tensor, TensorStorage, ConstTensor, shapeOfNNull

from google.protobuf.internal.containers import RepeatedCompositeFieldContainer
import numpy as np
from numpy import dtypes
from typing import List, Dict, Union

from onnx import AttributeProto, ModelProto, TensorProto, TypeProto, NodeProto

try:
	from onnx.helper import tensor_dtype_to_np_dtype
except ImportError:
	from onnx.mapping import TENSOR_TYPE_TO_NP_TYPE

	tensor_dtype_to_np_dtype = lambda x: TENSOR_TYPE_TO_NP_TYPE[x]

import functools
import typing
import operator


def prod[T](x: typing.Iterable[T]) -> Union[T, int]:
	return functools.reduce(operator.mul, x, 1)


# src: onnx/mapping.py
# not supported: STRING = 8 COMPLEX64 = 14, COMPLEX128 = 15
# NOTE: 17, 18, 19, 20 are float8, 10 is half
DTYPE_MAP = {
	1: np.float32,
	2: np.uint8,
	3: np.int8,
	4: np.uint16,
	5: np.int16,
	6: np.int32,
	7: np.int64,
	9: bool,
	10: np.float32,
	11: np.float64,
	12: np.uint32,
	13: np.uint64,
	16: np.float32,
	17: np.float32,
	18: np.float32,
	19: np.float32,
	20: np.float32,
}
# TODO: fix buffer_parse to use this and fix get_weight_and_biases to only use buffer_parse
import importlib

onnx_ops = importlib.import_module('.onnx_ops', __name__)


def get_as_const(x: Tensor) -> np.ndarray:
	assert isinstance(x, ConstTensor), f'not const {x} {x._tb}'
	return x.compute()


def _op_split(opt: dict, inp: list[Tensor], n: NodeProto) -> tuple[Tensor, ...]:
	axis = opt.get('axis', 0)
	splitting = inp[0]
	if len(inp) == 1:
		split = [shapeOfNNull(splitting)[axis] // len(n.output)] * len(n.output)
		for i in range(shapeOfNNull(splitting)[axis] % len(n.output)):
			split[i] += 1
		split = np.array(split)
	else:
		split = get_as_const(inp[1])

	ret: list[Tensor]
	i, ret = 0, []
	arg_1: list[None | tuple[int, int]] = [None] * splitting.ndim
	for s in split:
		arg_1[axis] = (i, i + s)
		ret.append(splitting.shrink(arg=tuple(arg_1)))
		i = i + s
	return tuple(ret)


def _op_slice(onnx_model_version, opt: dict, inp: list[Tensor]) -> Tensor:
	slicing_tensor = inp[0]
	if onnx_model_version < 10:
		axes = slicing_tensor._store.const(
			np.array(opt.get('axes', range(inp[0].ndim)), dtype=np.int32)
		)
		ends = slicing_tensor._store.const(np.array(opt['ends'], dtype=np.int32))
		starts = slicing_tensor._store.const(np.array(opt['starts'], dtype=np.int32))
		steps = slicing_tensor._store.const(
			np.array([1] * slicing_tensor.ndim, dtype=np.int32)
		)
	else:
		starts, ends = inp[1:3]
		if len(inp) <= 3:
			axes = slicing_tensor._store.const(
				np.array(range(slicing_tensor.ndim), dtype=np.int32)
			)
		else:
			axes = inp[3].cast(np.int32)

		if len(inp) > 4:
			steps = inp[4].cast(np.int32)
		else:
			steps = slicing_tensor._store.const(
				np.array([1] * slicing_tensor.ndim, dtype=np.int32)
			)

	def compute(
		slicing_tensor=slicing_tensor, axes=axes, ends=ends, starts=starts, steps=steps
	):
		slicing_tensor = slicing_tensor.compute()
		axes = axes.compute()
		ends = ends.compute()
		starts = starts.compute()
		steps = steps.compute()
		arg: list[tuple[int, int, int]] = [(0, x, 1) for x in slicing_tensor.shape]
		for i, axis in enumerate(axes):
			axis = int(axis) + slicing_tensor.ndim if axis < 0 else int(axis)
			if starts[i] < 0:
				starts[i] += slicing_tensor.shape[axis]
			if ends[i] < 0:
				ends[i] += slicing_tensor.shape[axis]
			starts[i], ends[i] = (
				max(0, min(starts[i], slicing_tensor.shape[axis])),
				max(0, min(ends[i], slicing_tensor.shape[axis])),
			)
			if starts[i] > ends[i] and steps[i] >= 0:
				steps[i] = -steps[i]
			arg[axis] = (starts[i], ends[i], steps[i])

		# new_shape = tuple((s, e) if st > 0 else (e + 1, s + 1) for s, e, st in arg)
		# if any(s == e for s, e in new_shape):
		# return slicing_tensor.shrink(new_shape)

		def unwrap(x):
			if isinstance(x, np.ndarray) and prod(x.shape) == 1:
				return x.reshape((1,))[0]
			return x

		return slicing_tensor[
			tuple([slice(unwrap(s), unwrap(e), unwrap(st)) for s, e, st in arg])
		]

	return slicing_tensor._store.new(None, slicing_tensor.dtype, compute)


def get_run_onnx(
	onnx_model: ModelProto,
	user_inputs: dict[str, Tensor],
	tensor_storage: TensorStorage,
):
	def type_parse(type_proto: TypeProto) -> tuple[int, ...]:
		ret = []
		while True:
			attr = type_proto.WhichOneof('value')
			if attr == 'tensor_type':
				if 'dim_value' not in type_proto.tensor_type.shape.dim.__dir__():
					return ()  # variable type, unable to determine shape
				elif not ret:
					return tuple([x.dim_value for x in type_proto.tensor_type.shape.dim])
				else:
					ret.extend([(x.dim_value,) for x in type_proto.tensor_type.shape.dim])
					return tuple(ret)
			elif attr == 'sequence_type':
				type_proto = getattr(type_proto, attr).elem_type
				ret.append(1)
			elif attr == 'map_type':
				raise NotImplementedError(f'map_type is not implemented: {type_proto}')
			elif attr == 'opaque_type':
				raise NotImplementedError(f'opaque_type is not implemented: {type_proto}')
			elif attr == 'sparse_tensor_type':
				raise NotImplementedError(
					f'sparse_tensor_type is not implemented: {type_proto}'
				)
			elif attr == 'optional_type':
				type_proto = getattr(type_proto, attr).elem_type
			else:
				raise Exception(f'unknown attr: {attr}, {type_proto}')

	def buffer_parse(inp: TensorProto) -> Tensor:
		if inp.data_type in (8, 14, 15):
			raise Exception(f'data type not supported {inp.name} {inp.dims} {inp.data_type}')
		dtype = DTYPE_MAP[inp.data_type]
		if dat := list(inp.float_data) or list(inp.int32_data) or list(inp.int64_data):
			return tensor_storage.const(np.array(dat, dtype=dtype).reshape(inp.dims))
		if len(inp.raw_data) > 0:
			return tensor_storage.const(
				np.frombuffer(inp.raw_data, dtype=tensor_dtype_to_np_dtype(inp.data_type))
				.copy()
				.astype(dtype)
				.reshape(tuple(inp.dims))
			)
		assert False

	def attribute_parse(
		a: AttributeProto,
	) -> (
		float | int | str | Tensor | tuple[float, ...] | tuple[int, ...] | tuple[str, ...]
	):
		if a.type == AttributeProto.FLOAT:
			return float(a.f)
		elif a.type == AttributeProto.INT:
			return int(a.i)
		elif a.type == AttributeProto.STRING:
			return a.s.decode('utf-8')
		elif a.type == AttributeProto.TENSOR:
			return buffer_parse(a.t)  # TENSOR
		elif a.type == AttributeProto.FLOATS:
			return tuple(float(x) for x in a.floats)
		elif a.type == AttributeProto.INTS:
			return tuple(int(x) for x in a.ints)
		elif a.type == AttributeProto.STRINGS:
			return tuple(x.decode('utf-8') for x in a.strings)
		elif a.type == AttributeProto.GRAPH:
			raise Exception(
				f'graph not implemented: {a.g}\n likely an OP requiring control flow'
			)
		else:
			raise Exception(f"can't parse {a.type} {a}")

	def attribute_to_dict(a: RepeatedCompositeFieldContainer[AttributeProto]):
		return {x.name: attribute_parse(x) for x in a}

	tensors: Dict[str, Tensor] = {}

	attribute_dict = {}
	for num, n in enumerate(onnx_model.graph.node):
		attribute_dict[num] = attribute_to_dict(n.attribute)

	# get weights and biases
	for inp_init in onnx_model.graph.initializer:
		tensors[inp_init.name] = buffer_parse(inp_init)

	def get_inputs():
		for inp in onnx_model.graph.input:
			if inp.name in tensors:
				continue
			shape = type_parse(inp.type)
			inp_tensor = user_inputs[inp.name]
			assert (
				inp_tensor.shape == shape or shape == ()
			), f'user {inp_tensor.shape} ;; file {shape}'
			tensors[inp.name] = inp_tensor

	get_inputs()

	onnx_model_version = onnx_model.opset_import[0].version
	intermediate_tensors: Dict[str, Tensor] = {}
	output_tensor_names = [x.name for x in onnx_model.graph.output]

	def fetch_tensor(x: str):
		if x in tensors:
			return tensors[x]
		if x in intermediate_tensors:
			return intermediate_tensors[x]
		return None

	for num, n in enumerate(onnx_model.graph.node):
		inp: List[Tensor] = []

		for x in n.input:
			t = fetch_tensor(x)
			assert t is not None
			inp.append(t)
		opt: Dict = attribute_dict[num]

		if n.op_type == 'Split':
			ret = _op_split(opt, inp, n)
		elif n.op_type == 'Slice':
			ret = _op_slice(onnx_model_version, opt, inp)
		elif hasattr(onnx_ops, n.op_type):
			fxn = getattr(onnx_ops, n.op_type)
			if isinstance(fxn, dict):
				for k in sorted(fxn.keys()):
					if k <= onnx_model_version:
						real_fxn = fxn[k]
			else:
				real_fxn = fxn
			import inspect

			if 'storage' in inspect.getfullargspec(real_fxn).kwonlyargs:
				ret = real_fxn(*inp, storage=tensor_storage, **opt)
			else:
				ret = real_fxn(*inp, **opt)
		else:
			raise Exception(f'op_type {n.op_type} not supported')

		if not isinstance(ret, tuple):
			ret = (ret,)
		assert len(n.output) <= len(
			ret
		), f"expected output size must be less than {len(ret)}, it's {n.output}"

		for i in range(len(n.output)):
			intermediate_tensors[n.output[i]] = ret[i]

	return {
		output_name: intermediate_tensors[output_name]
		for output_name in output_tensor_names
	}
