#!/usr/bin/env python3

from pathlib import Path
import sys

sys.path.append(str(Path(__file__).parent.parent))
from fuzz_common import do_fuzzing, StopFuzzingException, FuzzerBuilder

import numpy as np
import typing
from genlayer_embeddings import VecDB, EuclideanDistanceSquared
from genlayer.py.types import u32

from genlayer.py.storage import inmem_allocate
import itertools


NO_PARENT = 2**32 - 1


def check_cover_tree_invariants(db: VecDB) -> None:
	"""
	Check Cover Tree invariants and exit on first violation:
	1. Separating invariant: nodes at level i are at least base^i apart
	2. Covering invariant: every node at level i-1 is within base^i of some node at level i
	"""
	if db._root_idx == NO_PARENT or len(db) == 0:
		return

	# Group nodes by level
	nodes_by_level: dict[
		int, list[tuple[int, int]]
	] = {}  # level -> [(node_idx, element_id)]

	# Traverse all nodes
	stack = [db._root_idx]
	while stack:
		node_idx = stack.pop()
		if node_idx in db._free_nodes:
			continue
		node = db._nodes[node_idx]
		level = int(node.level)

		if level not in nodes_by_level:
			nodes_by_level[level] = []
		nodes_by_level[level].append((node_idx, int(node.element_id)))

		# Add children to stack
		for i in range(len(node.children)):
			stack.append(node.children[i])

	def print_struct():
		print(f'  Tree structure:')
		for lvl, lvl_nodes in sorted(nodes_by_level.items()):
			print(f'    Level {lvl}: {[elem_id for _, elem_id in lvl_nodes]}')

	# Check separating invariant
	for level, nodes in nodes_by_level.items():
		min_distance = db._base**level
		for i, (node_idx1, elem_id1) in enumerate(nodes):
			for j, (node_idx2, elem_id2) in enumerate(nodes):
				if i >= j:  # Skip self and duplicates
					continue

				if elem_id1 in db._free_nodes or elem_id2 in db._free_nodes:
					continue

				distance = db._dist_func(u32(elem_id1), u32(elem_id2))
				if distance < min_distance:
					print(f'SEPARATING INVARIANT VIOLATION at level {level}:')
					print(f'  Nodes {elem_id1} and {elem_id2} are {distance:.6f} apart')
					print(
						f'  Should be >= {min_distance:.6f} (base^{level} = {db._base}^{level})'
					)
					print(f'  Node {elem_id1} key: {db._elems[elem_id1].node_id}')
					print(f'  Node {elem_id2} key: {db._elems[elem_id2].node_id}')
					print_struct()

					for in_level in nodes_by_level.values():
						for node_idx, elem_id in in_level:
							print(
								f'      Node {elem_id} at level {db._nodes[node_idx].level} with key {db._elems[elem_id].node_id}'
							)

					assert False

	# Check covering invariant
	sorted_levels = sorted(nodes_by_level.keys())
	for i, level in enumerate(sorted_levels[:-1]):  # Skip highest level
		next_level = sorted_levels[i + 1]
		max_distance = db._base**next_level

		lower_nodes = nodes_by_level[level]
		higher_nodes = nodes_by_level[next_level]

		for node_idx1, elem_id1 in lower_nodes:
			if elem_id1 in db._free_nodes:
				continue

			# Find minimum distance to any node at higher level
			min_dist_to_higher = float('inf')
			covering_node = None
			for node_idx2, elem_id2 in higher_nodes:
				if elem_id2 in db._free_nodes:
					continue
				distance = db._dist_func(u32(elem_id1), u32(elem_id2))
				if distance < min_dist_to_higher:
					min_dist_to_higher = distance
					covering_node = elem_id2

			if min_dist_to_higher > max_distance:
				print(f'COVERING INVARIANT VIOLATION:')
				print(
					f'  Node {elem_id1} at level {level} is {min_dist_to_higher:.6f} from nearest higher node {covering_node}'
				)
				print(
					f'  Should be <= {max_distance:.6f} (base^{next_level} = {db._base}^{next_level}), but is {min_dist_to_higher:.6f}'
				)
				print(f'  Node {elem_id1} key: {db._elems[elem_id1].node_id}')
				print(
					f'  Covering node {covering_node} key: {db._elems[covering_node].node_id}'
				)

				print_struct()
				assert False


class Etalon:
	data: np.ndarray[tuple[int, typing.Literal[5]], np.dtype[np.float32]]
	vals: list[u32]

	def __init__(self):
		self.data = np.empty((0, 5), dtype=np.float32)
		self.vals = []

	def add(self, key: np.ndarray[tuple[typing.Literal[5]], np.dtype[np.float32]], val):
		self.data = np.vstack([self.data, key])
		self.vals.append(val)


def vec_db_2(buf):
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
		db = inmem_allocate(
			VecDB[np.float32, typing.Literal[5], u32, EuclideanDistanceSquared]
		)

		id_to_value: dict[VecDB.Id, u32] = {}

		cnt = builder.fetch(1)[0] % 80 + 10

		steps = []

		for i in range(cnt):
			c = builder.fetch(1)[0] % 3
			if len(db) == 0 and c == 0:
				c = 1

			match c:
				case 0:
					db_id, val = id_to_value.popitem()
					elem = db.get_by_id(db_id)
					steps.append(f'Remove element {elem.value}')
					elem.remove()
					rem_idx = etalon.vals.index(val)
					etalon.data = np.delete(etalon.data, rem_idx, axis=0)
					etalon.vals.pop(rem_idx)

					try:
						check_cover_tree_invariants(db)
					except AssertionError:
						print('=== steps ===')
						for step in steps:
							print(step)
						raise
				case 1:
					key = gen_vec()
					db_id = db.insert(key, u32(i))
					etalon.add(key, u32(i))

					id_to_value[db_id] = u32(i)

					steps.append(f'Add element {key}')

					try:
						check_cover_tree_invariants(db)
					except AssertionError:
						print('=== steps ===')
						for step in steps:
							print(step)
						raise
				case 2:
					query_around = gen_vec()

					k = builder.fetch(1)[0] % 3 + 3

					got = list((x.distance, x.value) for x in db.knn(query_around, k))
					got.sort(key=lambda x: x[0])

					d = EuclideanDistanceSquared()
					distances = d.batch(etalon.data, query_around)
					closest_indices = np.argsort(distances)[:k]

					exp = list((distances[i], etalon.vals[i]) for i in closest_indices)
					exp.sort(key=lambda x: x[0])

					norm = np.linalg.norm(
						np.array(list(x[0] for x in exp)) - np.array(list(x[0] for x in got))
					)

					if norm > 1e-5:
						print(f'k: {k}')
						print(f'query: {query_around}')
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
	do_fuzzing(vec_db_2)
