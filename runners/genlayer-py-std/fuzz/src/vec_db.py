#!/usr/bin/env python3

from pathlib import Path
import sys

sys.path.append(str(Path(__file__).parent.parent))
from fuzz_common import do_fuzzing, StopFuzzingException, FuzzerBuilder

import numpy as np
import typing
from genlayer_embeddings import VecDB
from genlayer.py.types import u32

from genlayer.py.storage import inmem_allocate
import itertools


class Etalon:
	data: np.ndarray[tuple[int, typing.Literal[5]], np.dtype[np.float32]]
	vals: list[u32]

	def __init__(self):
		self.data = np.empty((0, 5), dtype=np.float32)
		self.vals = []

	def add(self, key: np.ndarray[tuple[typing.Literal[5]], np.dtype[np.float32]], val):
		self.data = np.vstack([self.data, key])
		self.vals.append(val)


def vec_db(buf):
	builder = FuzzerBuilder(buf)

	def finite_float(num):
		i = 0
		while i < num:
			f = builder.fetch_float()
			if not np.isfinite(f):
				continue
			if abs(f) > 1e5:
				f = np.fmod(f, 1e5 + 3)
			yield f
			i += 1

	def gen_vec() -> np.ndarray:
		return np.array(list(finite_float(5))).astype(np.float32)

	try:
		etalon = Etalon()
		db = inmem_allocate(VecDB[np.float32, typing.Literal[5], u32])

		cnt = builder.fetch(1)[0] % 30 + 10

		for i in range(cnt):
			key = gen_vec()
			db.insert(key, u32(i))
			etalon.add(key, u32(i))

		query_arond = gen_vec()

		k = builder.fetch(1)[0] % 3 + 3

		got = list((x.distance, x.value) for x in db.knn(query_arond, k))
		got.sort(key=lambda x: x[0])

		dot_products = np.dot(etalon.data, query_arond)
		norms = np.linalg.norm(etalon.data, axis=1) * np.linalg.norm(query_arond)
		similarities = dot_products / norms
		distances = 1 - similarities

		# Filter out non-finite distances
		finite_mask = np.isfinite(distances)
		finite_distances = distances[finite_mask]
		finite_indices = np.where(finite_mask)[0]

		if len(finite_distances) == 0:
			return  # Skip if no finite distances

		# Get k closest from finite distances only
		if len(finite_distances) < k:
			closest_finite_indices = np.arange(len(finite_distances))
		else:
			closest_finite_indices = np.argpartition(finite_distances, k)[:k]

		closest_indices = finite_indices[closest_finite_indices]

		exp = list((distances[i], etalon.vals[i]) for i in closest_indices)
		exp.sort(key=lambda x: x[0])

		norm = np.linalg.norm(
			np.array(list(x[0] for x in exp)) - np.array(list(x[0] for x in got))
		)

		if norm > 1e-5:
			print(f'k: {k}')
			print(f'query: {query_arond}')
			print(f'expected: {exp}')
			print(f'got: {got}')

			for x in db:
				print(f'  {x.key} -> {x.value}')
				for y in got:
					if x.value == y[1]:
						print(f'    ^^^^ found in got {y}')
				if any(x.value == y[1] for y in exp):
					for y in exp:
						if x.value == y[1]:
							print(f'    ^^^^ found in exp {y}')

			assert False

	except StopFuzzingException:
		return


if __name__ == '__main__':
	do_fuzzing(vec_db)
