# { "Depends": "py-genlayer:test" }
from genlayer import *


class Contract(gl.Contract):
	@gl.public.write
	def main(self, idx: int):
		ret = [
			Address(b'\x01' * 20),
			'abc',
			123,
			b'xyz',
			True,
			None,
			False,
			[Address(b'\x02' * 20), Address(b'\x03' * 20)],
			{'a': 1, 'b': [1, 2, 3]},
		]
		print(f'{idx} [0...{len(ret)})')
		return ret[idx]
