from __future__ import annotations

__all__ = ('VecDB', 'VecDBElement', 'Distance', 'EuclideanDistanceSquared')

from genlayer.py.storage import DynArray, TreeMap
from genlayer.py.types import u32

from genlayer.py.storage.annotations import allow_storage

import typing
import numpy as np
import math


class Distance(typing.Protocol):
	def __call__(self, l, r) -> typing.Any: ...


@allow_storage
class EuclideanDistanceSquared(Distance):
	def __call__(self, l, r):
		return np.sum((l - r) ** 2)

	def batch(self, l, r):
		return ((l - r) ** 2).sum(axis=1)


Id = typing.NewType('Id', int)
_Id = Id

NO_PARENT = u32(0xFFFFFFFF)  # Constant for no parent node


@allow_storage
class CoverTreeNode:
	"""A node in the cover tree structure"""

	element_id: u32
	level: u32
	children: DynArray[u32]  # Indices of child nodes
	parent: u32  # Index of parent node, NO_PARENT if root

	def __init__(self, element_id: u32, level: u32):
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


@allow_storage
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
	_root_idx: u32
	_base: float  # Base for cover tree levels (typically 1.3)
	_max_level: u32
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
		self._max_level = u32(0)

	def __len__(self) -> int:
		return len(self._keys) - len(self._free_idx)

	def get_by_id(self, id: Id) -> VecDBElement[T, S, V, None]:
		res = self.get_by_id_or_none(id)
		if res is None:
			raise KeyError(f'no element with id {id}')
		return res

	def get_by_id_or_none(self, id: Id) -> VecDBElement[T, S, V, None] | None:
		if u32(id) in self._free_idx:
			return None
		return VecDBElement(self, u32(id), None)

	def _distance(self, idx1: u32, idx2: u32) -> T:
		"""Compute distance between two elements by their indices"""
		return self._dist_func(self._keys[idx1], self._keys[idx2])

	def _distance_to_point(self, idx: u32, point: np.ndarray[tuple[S], np.dtype[T]]) -> T:
		"""Compute distance from element to query point"""
		return self._dist_func(self._keys[idx], point)

	def _allocate_node(self, element_id: u32, level: u32) -> u32:
		"""Allocate a new node and return its index"""
		if len(self._free_nodes) > 0:
			node_idx = self._free_nodes.popitem()[0]
			self._nodes[node_idx] = CoverTreeNode(element_id, level)
			return node_idx
		else:
			node = CoverTreeNode(element_id, level)
			self._nodes.append(node)
			return u32(len(self._nodes) - 1)

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
			idx = u32(len(self._keys) - 1)

		# Insert into cover tree
		self._insert_into_tree(idx)

		return Id(idx)

	def _insert_into_tree(self, new_idx: u32) -> None:
		"""Insert element into cover tree structure"""
		if self._root_idx == NO_PARENT:
			# First element becomes root
			self._root_idx = self._allocate_node(new_idx, u32(0))
			self._nodes[self._root_idx].parent = NO_PARENT
			self._max_level = u32(0)
			return

		# Find insertion level based on distance to nearest neighbor
		nearest_dist = float('inf')
		for i in range(len(self._keys)):
			if u32(i) in self._free_idx or i == new_idx:
				continue
			dist = self._distance(new_idx, u32(i))
			if dist < nearest_dist:
				nearest_dist = dist

		# Determine level for new node
		if nearest_dist == 0:
			level = 0
		else:
			print(nearest_dist, self._base)
			level = min(self._max_level, int(math.log(nearest_dist) / math.log(self._base)))
		level = max(level, 0)

		# Create new node
		new_node_idx = self._allocate_node(new_idx, u32(level))

		# Insert at appropriate level
		self._insert_node_at_level(new_node_idx, u32(level))

	def _insert_node_at_level(self, new_node_idx: u32, level: u32) -> None:
		"""Insert node at specified level in the tree"""
		if level > self._max_level:
			# Need to create new root
			old_root_idx = self._root_idx
			self._nodes[new_node_idx].level = u32(level + 1)
			self._nodes[new_node_idx].parent = NO_PARENT
			self._root_idx = new_node_idx
			if old_root_idx != NO_PARENT:
				self._nodes[new_node_idx].children.append(old_root_idx)
				self._nodes[old_root_idx].parent = new_node_idx
			self._max_level = u32(level + 1)
			return

		# Find best parent at level + 1
		parent_candidates: list[u32] = self._find_nodes_at_level(level + 1)
		if len(parent_candidates) == 0 and self._root_idx != NO_PARENT:
			parent_candidates.append(self._root_idx)

		best_parent_idx = NO_PARENT
		best_distance = float('inf')

		for candidate_idx in parent_candidates:
			dist = self._distance(
				self._nodes[candidate_idx].element_id, self._nodes[new_node_idx].element_id
			)
			if dist < best_distance:
				best_distance = dist
				best_parent_idx = candidate_idx

		if best_parent_idx != NO_PARENT:
			self._nodes[new_node_idx].parent = best_parent_idx
			self._nodes[best_parent_idx].children.append(new_node_idx)

	def _find_nodes_at_level(self, target_level: int) -> list[u32]:
		"""Find all node indices at specified level"""
		nodes: list[u32] = []
		if self._root_idx == NO_PARENT:
			return nodes

		stack: list[u32] = [self._root_idx]

		while len(stack) > 0:
			node_idx = stack.pop()
			node = self._nodes[node_idx]
			if node.level == target_level:
				nodes.append(node_idx)
			elif node.level > target_level:
				for i in range(len(node.children)):
					stack.append(node.children[i])

		return nodes

	def _remove_from_tree(self, idx: u32) -> None:
		"""Remove element from cover tree structure"""
		# Find and remove the node
		node_idx = self._find_node_by_id(idx)
		if node_idx == NO_PARENT:
			return

		node = self._nodes[node_idx]
		parent_idx = node.parent
		children_idxs: list[u32] = [node.children[i] for i in range(len(node.children))]

		if parent_idx != NO_PARENT:
			# Remove from parent's children list
			parent = self._nodes[parent_idx]
			for i in range(len(parent.children)):
				if parent.children[i] == node_idx:
					parent.children[i : i + 1] = []
					break

			# Reattach children to parent
			for child_idx in children_idxs:
				self._nodes[child_idx].parent = parent_idx
				parent.children.append(child_idx)
		elif node_idx == self._root_idx:
			# Removing root
			if len(children_idxs) > 0:
				# Promote highest level child to root
				best_child_idx = children_idxs[0]
				best_level = self._nodes[best_child_idx].level
				for i in range(1, len(children_idxs)):
					child_idx = children_idxs[i]
					if self._nodes[child_idx].level > best_level:
						best_level = self._nodes[child_idx].level
						best_child_idx = child_idx

				self._root_idx = best_child_idx
				self._nodes[best_child_idx].parent = NO_PARENT

				# Reattach other children
				for child_idx in children_idxs:
					if child_idx != best_child_idx:
						self._nodes[child_idx].parent = best_child_idx
						self._nodes[best_child_idx].children.append(child_idx)
			else:
				self._root_idx = NO_PARENT
				self._max_level = u32(0)

		# Free the node
		self._free_node(node_idx)

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

	def knn(
		self, v: np.ndarray[tuple[S], np.dtype[T]], k: int
	) -> typing.Iterator[VecDBElement[T, S, V, T]]:
		"""Find k nearest neighbors using cover tree"""
		self._do_init()

		if self._root_idx == NO_PARENT or k <= 0:
			return

		# Use a priority queue approach for efficiency
		candidates: list[tuple[T, u32]] = []  # Will store (distance, element_id) tuples

		# Traverse tree to find candidates
		stack: list[u32] = [self._root_idx]
		while len(stack) > 0:
			node_idx = stack.pop()
			if node_idx in self._free_nodes:
				continue
			node = self._nodes[node_idx]
			if node.element_id not in self._free_idx:
				dist = self._distance_to_point(node.element_id, v)
				if np.isfinite(dist):
					candidates.append((dist, node.element_id))
			for i in range(len(node.children)):
				stack.append(node.children[i])

		# Sort by distance
		candidates.sort(key=lambda x: x[0])

		count = 0
		for dist, idx in candidates:
			if count >= k:
				break
			yield VecDBElement(self, idx, dist)
			count += 1

	def __iter__(self):
		self._do_init()

		for i in range(len(self._keys)):
			if u32(i) in self._free_idx:
				continue
			yield VecDBElement(self, u32(i), None)
