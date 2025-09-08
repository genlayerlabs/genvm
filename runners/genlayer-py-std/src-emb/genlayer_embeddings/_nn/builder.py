import typing
import numpy as np
import struct
import base64

prelude = """
import numpy as np
import struct
import base64, zlib

def extra_erf(x):
	t = 1.0 / (1.0 + 0.3275911 * np.abs(x))
	term1 = 0.254829592 * t
	term2 = -0.284496736 * t**2
	term3 = 1.421413741 * t**3
	term4 = -1.453152027 * t**4
	term5 = 1.061405429 * t**5
	y = term1 + term2 + term3 + term4 + term5
	z = 1.0 - y * np.exp(-x * x)
	return np.where(x > 0, z, -z)

import copy
import functools, operator

def prod(x):
	return functools.reduce(operator.mul, x, 1)

def _do_slicing(slicing_tensor, axes, ends, starts, steps):
	starts = copy.copy(starts)
	ends = copy.copy(ends)
	steps = copy.copy(steps)
	axes = copy.copy(axes)
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

	def unwrap(x):
		if isinstance(x, np.ndarray) and prod(x.shape) == 1:
			return x.reshape((1,))[0]
		return x

	return slicing_tensor[
		tuple([slice(unwrap(s), unwrap(e), unwrap(st)) for s, e, st in arg])
	]
"""


class Builder:
	_data: list[str]
	_globals: dict[str, typing.Any]
	_consts: dict[int, str]

	def __init__(self, name: str, compress: bool = False):
		self._name = name
		self._globals = {}
		self._prelude = [prelude]
		self._data = []
		self._next_const = 0
		self._next_val = 0
		self._consts = {}
		self._compress = compress

	def add_decl(self, expr) -> str:
		name = f'v{self._next_val}'
		self._next_val += 1
		self._data.append(f'{name} = ({expr})\n')
		return name

	def add_const(self, v) -> str:
		if isinstance(v, int):
			return str(v)
		if got := self._consts.get(id(v)):
			return got
		name = f'c{self._next_const}'
		self._next_const += 1
		if isinstance(v, np.ndarray):
			v = v.copy()

			as_bytes = v.tobytes()
			if len(as_bytes) > 128:
				if self._compress:
					import zlib

					new_bytes = zlib.compress(as_bytes, level=9)
					as_str = f'zlib.decompress(base64.b64decode({repr(base64.b64encode(new_bytes).decode("ascii"))}))'
				else:
					as_str = (
						f'base64.b64decode({repr(base64.b64encode(as_bytes).decode("ascii"))})'
					)
			else:
				as_str = repr(as_bytes)

			self._prelude.append(
				f'{name} = np.frombuffer({as_str}, dtype=np.{v.dtype}).reshape({v.shape})\n'
			)
			# self._prelude.append(f'# ^ {'\n# '.join(repr(v).split('\n'))}\n')

			self._globals[name] = v
		elif isinstance(v, np.number):
			self._prelude.append(
				f'{name} = np.frombuffer({v.tobytes()}, dtype=np.{v.dtype})[0]\n'
			)
		elif isinstance(v, float):
			self._prelude.append(f'{name} = struct.unpack(\'d\', {struct.pack('d', v)})[0]\n')
		else:
			self._globals[name] = v
		self._consts[id(v)] = name
		return name

	def finish_str(self, parameters: list[str] = []) -> str:
		d = ''.join(self._data)
		params_str = ', '.join(parameters)
		if not d.strip():
			d = '\tpass\n'
		else:
			d = '\n'.join('\t' + line for line in d.split('\n') if line.strip()) + '\n'

		self._prelude.append(f'def main({params_str}):\n')
		d = ''.join(self._prelude) + d

		return d

	def finish(self, parameters: list[str] = []) -> typing.Callable:
		d = self.finish_str(parameters)
		code_obj = compile(d, self._name, 'exec')
		exec(code_obj, self._globals)
		return self._globals['main']
