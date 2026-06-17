#!/usr/bin/env python3

from pathlib import Path
import sys

sys.path.append(str(Path(__file__).parent.parent))
from fuzz_common import do_fuzzing, StopFuzzingException, FuzzerBuilder

import numpy as np
import typing
from genlayer_embeddings import VecDB, EuclideanDistance
from genlayer_embeddings.vecdb import NO_PARENT, MIN_LEVEL, MAX_LEVEL
from genlayer.types import u32

from genlayer.storage import inmem_allocate
import itertools


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

	# Check root is not freed
	if db._root_idx in db._free_nodes:
		print(f'STRUCTURAL VIOLATION: root node {db._root_idx} is in free nodes')
		assert False

	# Traverse all nodes
	stack = [db._root_idx]
	while stack:
		node_idx = stack.pop()
		node = db._nodes[node_idx]
		level = int(node.level)
		elem_id = int(node.element_id)

		# Structural check: active node must not reference freed element
		if elem_id in db._free_idx:
			print(
				f'STRUCTURAL VIOLATION: active node {node_idx} references freed element {elem_id}'
			)
			assert False

		if level not in nodes_by_level:
			nodes_by_level[level] = []
		nodes_by_level[level].append((node_idx, elem_id))

		# Add children to stack, checking no freed children
		for i in range(len(node.children)):
			child_idx = node.children[i]
			if child_idx in db._free_nodes:
				print(
					f'STRUCTURAL VIOLATION: node {node_idx} (element {elem_id}, level {level}) has freed child {child_idx}'
				)
				assert False
			stack.append(child_idx)

	def print_struct():
		print(f'  Tree structure:')
		for lvl, lvl_nodes in sorted(nodes_by_level.items()):
			print(f'    Level {lvl}: {[elem_id for _, elem_id in lvl_nodes]}')

	# Check children are strictly below their parent
	stack2 = [db._root_idx]
	while stack2:
		parent_idx = stack2.pop()
		parent_node = db._nodes[parent_idx]
		for i in range(len(parent_node.children)):
			cidx = parent_node.children[i]
			stack2.append(cidx)
			actual_level = int(db._nodes[cidx].level)
			if actual_level >= int(parent_node.level):
				eid = int(db._nodes[cidx].element_id)
				print(f'LEVEL ORDER VIOLATION:')
				print(
					f'  Child {eid} at level {actual_level}, parent {parent_node.element_id} at level {parent_node.level}'
				)
				print_struct()
				assert False

	# Check local separating invariant: children of the same parent must be
	# separated at the child level. With level gaps, the global separating
	# invariant (all nodes in C_i) cannot be maintained without intermediate
	# nodes, but local separation ensures bounded branching factor.
	stack3 = [db._root_idx]
	while stack3:
		parent_idx = stack3.pop()
		parent_node = db._nodes[parent_idx]
		children = []
		for i in range(len(parent_node.children)):
			cidx = parent_node.children[i]
			stack3.append(cidx)
			children.append(cidx)
		for i, cidx1 in enumerate(children):
			child1 = db._nodes[cidx1]
			eid1 = int(child1.element_id)
			lvl1 = int(child1.level)
			for j, cidx2 in enumerate(children):
				if i >= j:
					continue
				child2 = db._nodes[cidx2]
				eid2 = int(child2.element_id)
				lvl2 = int(child2.level)
				if eid1 == eid2:
					continue
				check_level = min(lvl1, lvl2)
				if check_level <= MIN_LEVEL or check_level >= MAX_LEVEL:
					continue
				min_distance = db._base**check_level
				distance = db._dist_func(db._keys[eid1], db._keys[eid2])
				if distance < min_distance:
					print(f'LOCAL SEPARATING INVARIANT VIOLATION:')
					print(
						f'  Siblings {eid1} (level {lvl1}) and {eid2} (level {lvl2}) under parent {parent_node.element_id} (level {parent_node.level})'
					)
					print(f'  Distance {distance:.6f} < {min_distance:.6f} (base^{check_level})')
					print_struct()
					assert False

	# Check covering invariant (parent-child: d(parent, child) <= base^parent_level)
	stack = [db._root_idx]
	while stack:
		node_idx = stack.pop()
		node = db._nodes[node_idx]
		for i in range(len(node.children)):
			child_idx = node.children[i]
			child_node = db._nodes[child_idx]
			parent_key = db._keys[int(node.element_id)]
			child_key = db._keys[int(child_node.element_id)]
			distance = db._dist_func(parent_key, child_key)
			if int(node.level) <= MIN_LEVEL or int(node.level) >= MAX_LEVEL:
				stack.append(child_idx)
				continue  # clamped sentinel levels; skip
			max_distance = db._base ** int(node.level)
			if distance > max_distance:
				print(f'COVERING INVARIANT VIOLATION:')
				print(
					f'  Child {child_node.element_id} at level {child_node.level} is {distance:.6f} from parent {node.element_id} at level {node.level}'
				)
				print(
					f'  Should be <= {max_distance:.6f} (base^{node.level} = {db._base}^{node.level})'
				)
				print(f'  Child key: {child_key}')
				print(f'  Parent key: {parent_key}')
				print_struct()
				assert False
			stack.append(child_idx)


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
		db = inmem_allocate(VecDB[np.float32, typing.Literal[5], u32, EuclideanDistance])

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
					db_id = db.insert(key, i)
					etalon.add(key, i)

					id_to_value[db_id] = i

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

					d = EuclideanDistance()
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
