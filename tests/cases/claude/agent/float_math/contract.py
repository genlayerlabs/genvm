# { "Depends": "py-genlayer:test" }
import genlayer as gl

import math


class Contract(gl.contract.Contract):
	def __init__(self):
		pass

	@gl.public.view
	def compute(self) -> str:
		# Transcendental + irrational results are the classic place where a hardware
		# FPU's last bit / x87-80bit intermediates leak per-machine nondeterminism.
		# In det mode these must go through the deterministic softfloat path.
		acc = 0.0
		for i in range(1, 50):
			x = float(i) * 0.123456789
			acc += math.sin(x) * math.cos(x)
			acc += math.exp(x % 3.0)
			acc += math.sqrt(x) * math.log(x + 1.0)
			acc += math.atan2(x, 1.0 + x) ** 1.5
		# repr() of a float is itself a place divergence can hide
		return f'{acc!r}|{math.pi!r}|{(2.0 ** 0.5)!r}|{(0.1 + 0.2)!r}'
