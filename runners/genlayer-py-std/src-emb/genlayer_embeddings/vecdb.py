from __future__ import annotations

__all__ = (
	'VecDB',
	'VecDBElement',
	'Distance',
	'EuclideanDistance',
	'ManhattanDistance',
	'ChebyshevDistance',
)

from genlayer.storage import DynArray, TreeMap
from genlayer.types import u32, i32

from genlayer.storage import allow

import typing
import numpy as np
import math


class Distance(typing.Protocol):
	"""
	Protocol for distance functions used by :py:class:`VecDB`.

	Implementations must be a true metric (non-negative, symmetric, zero iff
	equal, and satisfying the triangle inequality); otherwise the cover-tree
	pruning in :py:meth:`VecDB.knn` may skip the true nearest neighbor.
	"""

	def __call__(self, l, r) -> typing.Any:
		"""
		Compute the distance between two vectors.

		:param l: left-hand vector
		:param r: right-hand vector
		:returns: distance between ``l`` and ``r``
		"""
		...


@allow
class EuclideanDistance(Distance):
	def __call__(self, l, r):
		return np.sqrt(np.sum((l - r) ** 2))

	def batch(self, l, r):
		return np.sqrt(((l - r) ** 2).sum(axis=1))


@allow
class ManhattanDistance(Distance):
	"""L1 (taxicab) distance. A true metric, safe for cover-tree pruning."""

	def __call__(self, l, r):
		return np.sum(np.abs(l - r))

	def batch(self, l, r):
		return np.abs(l - r).sum(axis=1)


@allow
class ChebyshevDistance(Distance):
	"""L-infinity (max-coordinate) distance. A true metric, safe for pruning."""

	def __call__(self, l, r):
		return np.max(np.abs(l - r))

	def batch(self, l, r):
		return np.abs(l - r).max(axis=1)


Id = typing.NewType('Id', int)
_Id = Id

NO_PARENT: u32 = 0xFFFFFFFF  # Constant for no parent node
MIN_LEVEL: i32 = -64  # Sentinel for coincident points in level calculation
MAX_LEVEL: i32 = 65535  # Cap for root level


@allow
class CoverTreeNode:
	"""A node in the cover tree structure"""

	element_id: u32
	level: i32
	children: DynArray[u32]  # Indices of child nodes
	parent: u32  # Index of parent node, NO_PARENT if root

	def __init__(self, element_id: u32, level: i32):
		self.element_id = element_id
		self.level = level


class VecDBElement[T: np.number, S: int, V, Dist]:
	distance: Dist
	"""
	Distance from search point to this element, if any
	"""

	__slots__ = ('_idx', '_db', 'distance')

	def __init__(self, db: VecDB[T, S, V], idx: u32, distance: Dist):
		self._idx = idx
		self._db = db
		self.distance = distance

	def __repr__(self) -> str:
		return f'VecDB.Element(id={self.id!r}, key={self.key!r}, value={self.value!r}, distance={self.distance})'

	@property
	def key(self) -> np.ndarray[tuple[S], np.dtype[T]]:
		"""
		Key (vector) of this element
		"""
		return self._db._keys[self._idx]

	@property
	def id(self) -> Id:
		"""
		Id (unique key) of this element
		"""
		return Id(self._idx)

	@property
	def value(self) -> V:
		"""
		Value of this element
		"""
		return self._db._values[self._idx]

	@value.setter
	def value(self, v: V):
		self._db._values[self._idx] = v

	def remove(self) -> None:
		"""
		Removes current element from the db
		"""
		self._db._remove_from_tree(self._idx)
		self._db._free_idx[self._idx] = None


@allow
class VecDB[T: np.number, S: int, V, D: Distance]:
	"""
	Data structure that supports storing and querying vector data using Cover Trees

	Cover trees provide logarithmic time nearest neighbor search with theoretical guarantees.

	There are two entities that can act as a key:

	#. vector (can have duplicates)
	#. id (int alias, can't have duplicates)

	.. warning::
		import :py:mod:`numpy` before ``from genlayer import *`` if you wish to use :py:class:`VecDB`!
	"""

	type Id = _Id
	"""
	:py:class:`int` alias to prevent confusion
	"""

	type Element = VecDBElement
	"""
	Shorthand to prevent global namespace pollution
	"""

	_keys: DynArray[np.ndarray[tuple[S], np.dtype[T]]]
	_values: DynArray[V]
	_free_idx: TreeMap[u32, None]
	_nodes: DynArray[CoverTreeNode]
	_free_nodes: TreeMap[u32, None]
	_elem_to_node: TreeMap[u32, u32]  # element_id -> highest-level node_idx
	_root_idx: u32
	_base: float  # Base for cover tree levels (typically 1.3)
	_max_level: i32
	_min_level: i32
	_dist_func: D

	_initialized: bool = False

	def __init__(self):
		self._do_init()

	def _do_init(self):
		if self._initialized:
			return
		self._initialized = True
		self._root_idx = NO_PARENT
		self._base = 1.3
		self._max_level = 0
		self._min_level = 0

	def __len__(self) -> int:
		return len(self._keys) - len(self._free_idx)

	def get_by_id(self, id: Id) -> VecDBElement[T, S, V, None]:
		res = self.get_by_id_or_none(id)
		if res is None:
			raise KeyError(f'no element with id {id}')
		return res

	def get_by_id_or_none(self, id: Id) -> VecDBElement[T, S, V, None] | None:
		if id < 0 or id >= len(self._keys):
			return None
		if id in self._free_idx:
			return None
		return VecDBElement(self, id, None)

	def _distance(self, idx1: u32, idx2: u32) -> T:
		"""Compute distance between two elements by their indices"""
		return self._dist_func(self._keys[idx1], self._keys[idx2])

	def _distance_to_point(self, idx: u32, point: np.ndarray[tuple[S], np.dtype[T]]) -> T:
		"""Compute distance from element to query point"""
		return self._dist_func(self._keys[idx], point)

	def _allocate_node(self, element_id: u32, level: i32) -> u32:
		"""Allocate a new node and return its index"""
		if len(self._free_nodes) > 0:
			node_idx = self._free_nodes.popitem()[0]
			self._nodes[node_idx] = CoverTreeNode(element_id, level)
			return node_idx
		else:
			node = CoverTreeNode(element_id, level)
			self._nodes.append(node)
			return len(self._nodes) - 1

	def _free_node(self, node_idx: u32) -> None:
		"""Mark a node as free"""
		self._free_nodes[node_idx] = None

	def insert(self, key: np.ndarray[tuple[S], np.dtype[T]], val: V) -> Id:
		self._do_init()
		# Add to storage arrays
		if len(self._free_idx) > 0:
			idx = self._free_idx.popitem()[0]
			self._keys[idx] = key
			self._values[idx] = val
		else:
			self._keys.append(key)
			self._values.append(val)
			idx = len(self._keys) - 1

		# Insert into cover tree
		self._insert_into_tree(idx)

		return Id(idx)

	def _level_for_dist(self, dist: float) -> i32:
		"""Compute the cover tree level for a given distance.

		Returns largest L such that base^L < dist (i.e. dist > base^L).
		This ensures the separating invariant: points at level L are > base^L apart.
		"""
		if dist <= 0:
			return MIN_LEVEL  # coincident points: clamp to deepest level
		return int(math.ceil(math.log(dist) / math.log(self._base))) - 1

	def _insert_into_tree(self, new_idx: u32) -> None:
		"""Insert element into cover tree structure using top-down descent"""
		if self._root_idx == NO_PARENT:
			# First element becomes root
			self._root_idx = self._allocate_node(new_idx, 0)
			self._nodes[self._root_idx].parent = NO_PARENT
			self._max_level = 0
			self._min_level = 0
			self._elem_to_node[new_idx] = self._root_idx
			return

		# If new point is too far from root, make it the new root
		root_node = self._nodes[self._root_idx]
		root_dist = float(self._distance(new_idx, root_node.element_id))
		if root_dist > self._base ** int(root_node.level):
			needed_level = int(math.ceil(math.log(root_dist) / math.log(self._base)))
			needed_level = max(needed_level, int(root_node.level) + 1)
			needed_level = min(needed_level, MAX_LEVEL)

			new_root_idx = self._allocate_node(new_idx, needed_level)
			self._nodes[new_root_idx].parent = NO_PARENT
			self._nodes[new_root_idx].children.append(self._root_idx)
			self._nodes[self._root_idx].parent = new_root_idx

			self._root_idx = new_root_idx
			self._max_level = needed_level
			self._elem_to_node[new_idx] = new_root_idx
			return

		# Candidate-set descent: track ALL covering nodes, not just one path.
		# This ensures the insertion level satisfies the separating invariant
		# against every covering ancestor, not just the immediate parent.
		candidates: list[tuple[u32, float]] = [(self._root_idx, root_dist)]
		min_cover_dist = root_dist

		while True:
			next_covering: list[tuple[u32, float]] = []
			for nidx, _ in candidates:
				node = self._nodes[nidx]
				for i in range(len(node.children)):
					child_idx = node.children[i]
					child_node = self._nodes[child_idx]
					if int(child_node.level) <= MIN_LEVEL:
						continue
					dist = float(self._distance(new_idx, child_node.element_id))
					if dist <= self._base ** int(child_node.level):
						next_covering.append((child_idx, dist))
						if dist < min_cover_dist:
							min_cover_dist = dist

			if not next_covering:
				break
			candidates = next_covering

		# Insert as child of nearest deepest covering node
		nearest_nidx = candidates[0][0]
		nearest_dist = candidates[0][1]
		for nidx, d in candidates:
			if d < nearest_dist:
				nearest_nidx = nidx
				nearest_dist = d

		new_level = self._level_for_dist(min_cover_dist)
		new_level = min(new_level, int(self._nodes[nearest_nidx].level) - 1)
		new_level = max(new_level, MIN_LEVEL)
		new_node_idx = self._allocate_node(new_idx, new_level)
		self._nodes[new_node_idx].parent = nearest_nidx
		self._nodes[nearest_nidx].children.append(new_node_idx)
		if new_level < self._min_level:
			self._min_level = new_level
		self._elem_to_node[new_idx] = new_node_idx

	def _remove_from_tree(self, idx: u32) -> None:
		"""Remove element from cover tree using swap-descent to a leaf.

		Instead of reinserting all descendants, we greedily descend from the
		deleted node to a leaf, swapping elements upward at each step.  Only
		the leaf is physically removed.  After the descent we walk back up
		and reinsert only the subtrees of children that violate the covering
		invariant due to the element swap.
		"""
		if idx in self._elem_to_node:
			node_idx = self._elem_to_node[idx]
			del self._elem_to_node[idx]
		else:
			# Fallback for legacy data without _elem_to_node populated
			node_idx = self._find_node_by_id(idx)
			if node_idx == NO_PARENT:
				return

		# Phase 1: descend from node_idx to a leaf, swapping elements up
		path: list[u32] = []  # nodes whose element was swapped (need violation check)
		current = node_idx

		while len(self._nodes[current].children) > 0:
			node = self._nodes[current]
			# Find the child whose element is nearest to current element
			nearest_child_idx = node.children[0]
			nearest_dist = float(
				self._distance(node.element_id, self._nodes[nearest_child_idx].element_id)
			)
			for i in range(1, len(node.children)):
				child_idx = node.children[i]
				d = float(self._distance(node.element_id, self._nodes[child_idx].element_id))
				if d < nearest_dist:
					nearest_dist = d
					nearest_child_idx = child_idx

			# Swap: current node adopts the nearest child's element
			replacement_elem = self._nodes[nearest_child_idx].element_id
			node.element_id = replacement_elem
			self._elem_to_node[replacement_elem] = current

			path.append(current)
			current = nearest_child_idx

		# `current` is now a leaf — remove it
		leaf = self._nodes[current]
		parent_idx = leaf.parent
		if parent_idx != NO_PARENT:
			parent = self._nodes[parent_idx]
			for i in range(len(parent.children)):
				if parent.children[i] == current:
					parent.children[i : i + 1] = []
					break
		elif current == self._root_idx:
			self._root_idx = NO_PARENT
			self._max_level = 0
			self._min_level = 0
		self._free_node(current)

		# Phase 2: walk back up, fixing covering-invariant violations
		for swap_idx in reversed(path):
			node = self._nodes[swap_idx]
			to_reinsert: list[u32] = []
			i = 0
			while i < len(node.children):
				child_idx = node.children[i]
				child_node = self._nodes[child_idx]
				d = float(self._distance(node.element_id, child_node.element_id))
				if d > self._base ** int(node.level):
					to_reinsert.append(child_idx)
					node.children[i : i + 1] = []
				else:
					i += 1
			for v_idx in to_reinsert:
				self._reinsert_subtree(v_idx)

	def _reinsert_subtree(self, node_idx: u32) -> None:
		"""Collect all elements from a detached subtree, free its nodes,
		and reinsert the elements into the tree."""
		elements: list[u32] = []
		stack: list[u32] = [node_idx]
		while len(stack) > 0:
			nidx = stack.pop()
			n = self._nodes[nidx]
			elements.append(n.element_id)
			for i in range(len(n.children)):
				stack.append(n.children[i])
			if n.element_id in self._elem_to_node:
				del self._elem_to_node[n.element_id]
			self._free_node(nidx)
		for eid in elements:
			self._insert_into_tree(eid)

	def _find_node_by_id(self, element_id: u32) -> u32:
		"""Find node index with given element ID"""
		if self._root_idx == NO_PARENT:
			return NO_PARENT

		stack: list[u32] = [self._root_idx]
		while len(stack) > 0:
			node_idx = stack.pop()
			if node_idx in self._free_nodes:
				continue
			node = self._nodes[node_idx]
			if node.element_id == element_id:
				return node_idx
			for i in range(len(node.children)):
				stack.append(node.children[i])

		return NO_PARENT

	def _max_descendant_dist(self, level: int) -> float:
		"""Upper bound on distance from a node at `level` to any descendant.

		Each ancestor at level l covers its child within base^l. Summing
		the geometric series from level down gives base^(level+1)/(base-1).
		"""
		return self._base ** (level + 1) / (self._base - 1)

	def knn(
		self, v: np.ndarray[tuple[S], np.dtype[T]], k: int
	) -> typing.Iterator[VecDBElement[T, S, V, T]]:
		"""Find k nearest neighbors using cover tree with pruning"""
		self._do_init()

		if self._root_idx == NO_PARENT or k <= 0:
			return

		import heapq

		# Max-heap of size k tracking the best candidates (neg_dist, element_id)
		best: list[tuple[float, u32]] = []

		def best_kth_dist() -> float:
			if len(best) < k:
				return float('inf')
			return -best[0][0]

		# DFS with pruning; stack entries: (node_idx, dist_to_query)
		root_node = self._nodes[self._root_idx]
		root_dist = float(self._distance_to_point(root_node.element_id, v))
		stack: list[tuple[u32, float]] = [(self._root_idx, root_dist)]

		while len(stack) > 0:
			node_idx, node_dist = stack.pop()
			if node_idx in self._free_nodes:
				continue
			node = self._nodes[node_idx]

			# Add this node's element to best-k
			if node.element_id not in self._free_idx and np.isfinite(node_dist):
				if len(best) < k:
					heapq.heappush(best, (-node_dist, node.element_id))
				elif node_dist < -best[0][0]:
					heapq.heapreplace(best, (-node_dist, node.element_id))

			# Collect children with distances, then sort farthest-first
			# so DFS pops the closest child first (better pruning)
			children_with_dist: list[tuple[float, u32]] = []
			for i in range(len(node.children)):
				child_idx = node.children[i]
				child_node = self._nodes[child_idx]
				child_dist = float(self._distance_to_point(child_node.element_id, v))
				# Prune: closest possible descendant is child_dist - max_descendant_dist
				mdd = self._max_descendant_dist(int(child_node.level))
				if child_dist - mdd <= best_kth_dist():
					children_with_dist.append((child_dist, child_idx))

			# Sort descending so closest is popped first from stack
			children_with_dist.sort(key=lambda x: -x[0])
			for child_dist, child_idx in children_with_dist:
				stack.append((child_idx, child_dist))

		# Yield results sorted by distance
		results = sorted((-d, eid) for d, eid in best)
		for dist, eid in results:
			yield VecDBElement(self, eid, dist)

	def __iter__(self):
		self._do_init()

		for i in range(len(self._keys)):
			if i in self._free_idx:
				continue
			yield VecDBElement(self, i, None)
