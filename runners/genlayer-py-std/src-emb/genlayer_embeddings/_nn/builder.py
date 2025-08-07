import typing
import numpy as np


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


class Builder:
	_data: list[str]
	_globals: dict[str, typing.Any]
	_consts: dict[int, str]

	def __init__(self, name: str):
		self._name = name
		self._globals = {
			'np': np,
			'extra_erf': extra_erf,
		}
		self._data = []
		self._next_const = 0
		self._next_val = 0
		self._consts = {}

	def add_decl(self, expr) -> str:
		name = f'v{self._next_val}'
		self._next_val += 1
		self._data.append(f'{name} = ({expr})\n')
		return name

	def add_const(self, v) -> str:
		if got := self._consts.get(id(v)):
			return got
		name = f'c{self._next_const}'
		self._next_const += 1
		# arr.setflags(write=False)
		self._globals[name] = v
		self._consts[id(v)] = name
		return name

	def finish(self, parameters: list[str] = []) -> typing.Callable:
		d = ''.join(self._data)
		params_str = ', '.join(parameters)
		if not d.strip():
			d = '\tpass\n'
		else:
			d = '\n'.join('\t' + line for line in d.split('\n') if line.strip())
		d = f'def main({params_str}):\n{d}'

		code_obj = compile(d, self._name, 'exec')
		exec(code_obj, self._globals)
		return self._globals['main']
