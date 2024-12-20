__all__ = ('TreeMap', 'Comparable')

import abc
import typing
import collections.abc

from .vec import DynArray
from genlayer.py.types import u32, i8


_NO_OBJ = object()


class _Node[K, V]:
	key: K
	value: V
	left: u32
	right: u32
	balance: i8

	def __init__(self, k: K, v: V):
		self.key = k
		if v is not _NO_OBJ:
			self.value = v
		self.left = u32(0)
		self.right = u32(0)
		self.balance = i8(0)


class Comparable(typing.Protocol):
	@abc.abstractmethod
	def __eq__(self, other: typing.Any, /) -> bool: ...

	@abc.abstractmethod
	def __lt__(self, other: typing.Any, /) -> bool: ...


class TreeMap[K: Comparable, V](collections.abc.MutableMapping[K, V]):
	"""
	Represents a mapping from keys to values that can be persisted on the blockchain

	``K`` must implement :py:class:`genlayer.py.storage.tree_map.Comparable` protocol ("<" and "=" are needed)
	"""

	_root: u32
	_slots: DynArray[_Node[K, V]]
	_free_slots: DynArray[u32]

	def __init__(self):
		"""
		This class can't be created with ``TreeMap()``

		:raises TypeError: always
		"""
		raise TypeError("this class can't be instantiated by user")

	def clear(self):
		self._root = u32(0)
		self._slots.clear()
		self._free_slots.clear()

	def __len__(self) -> int:
		return len(self._slots) - len(self._free_slots)

	def _alloc_slot(self) -> tuple[int, _Node[K, V]]:
		if len(self._free_slots) > 0:
			idx = int(self._free_slots[-1])
			self._free_slots.pop()
			slot = self._slots[idx]
		else:
			idx = len(self._slots)
			slot = self._slots.append_new_get()
		return (idx, slot)

	def _free_slot(self, slot: u32):
		if slot + 1 == len(self._slots):
			self._slots.pop()
		else:
			self._free_slots.append(slot)

	def _rot_left(self, par: int, cur: int):
		par_node = self._slots[par - 1]
		cur_node = self._slots[cur - 1]
		cur_l = cur_node.left
		cur_node.left = u32(par)
		par_node.right = cur_l
		if cur_node.balance == 0:
			par_node.balance = i8(+1)
			cur_node.balance = i8(-1)
		else:
			par_node.balance = i8(0)
			cur_node.balance = i8(0)

	def _rot_right(self, par: int, cur: int):
		par_node = self._slots[par - 1]
		cur_node = self._slots[cur - 1]
		cur_r = cur_node.right
		cur_node.right = u32(par)
		par_node.left = cur_r
		if cur_node.balance == 0:
			par_node.balance = i8(-1)
			cur_node.balance = i8(+1)
		else:
			par_node.balance = i8(0)
			cur_node.balance = i8(0)

	def _rot_right_left(self, gpar: int, par: int, cur: int):
		gpar_node = self._slots[gpar - 1]
		par_node = self._slots[par - 1]
		cur_node = self._slots[cur - 1]
		cur_l = cur_node.left
		cur_r = cur_node.right

		gpar_node.right = cur_l
		par_node.left = cur_r

		cur_node.left = u32(gpar)
		cur_node.right = u32(par)

		if cur_node.balance == 0:
			par_node.balance = i8(0)
			gpar_node.balance = i8(0)
		elif cur_node.balance > 0:
			gpar_node.balance = i8(-1)
			par_node.balance = i8(0)
		else:
			gpar_node.balance = i8(0)
			par_node.balance = i8(1)
		cur_node.balance = i8(0)

	def _rot_left_right(self, gpar: int, par: int, cur: int):
		gpar_node = self._slots[gpar - 1]
		par_node = self._slots[par - 1]
		cur_node = self._slots[cur - 1]
		cur_l = cur_node.left
		cur_r = cur_node.right

		gpar_node.left = cur_r
		par_node.right = cur_l

		cur_node.left = u32(par)
		cur_node.right = u32(gpar)

		if cur_node.balance == 0:
			par_node.balance = i8(0)
			gpar_node.balance = i8(0)
		elif cur_node.balance > 0:
			par_node.balance = i8(-1)
			gpar_node.balance = i8(0)
		else:
			par_node.balance = i8(0)
			gpar_node.balance = i8(1)
		cur_node.balance = i8(0)

	def _find_seq(self, k):
		seq = []
		cur = self._root
		is_less = True
		while True:
			seq.append(cur)
			if cur == 0:
				break
			cur_node = self._slots[cur - 1]
			if cur_node.key == k:
				break
			is_less = k < cur_node.key
			if is_less:
				cur = cur_node.left
			else:
				cur = cur_node.right
		return (seq, is_less)

	def __delitem__(self, k: K):
		seq, is_less = self._find_seq(k)
		# not found
		if seq[-1] == 0:
			raise KeyError('key not found')
		del_node = self._slots[seq[-1] - 1]
		del_left = del_node.left
		del_right = del_node.right
		del_balance = del_node.balance
		del del_node
		self._free_slot(seq[-1] - 1)

		special_null = False
		seq_move_to = len(seq) - 1
		# it has <=1 child
		if del_left == 0 or del_right == 0:
			if del_left == 0:
				seq[seq_move_to] = del_right
			else:
				seq[seq_move_to] = del_left
			special_null = True
		else:
			# we need to go right and then left*
			seq.append(del_right)
			while True:
				cur_node = self._slots[seq[-1] - 1]
				lft = cur_node.left
				if lft != 0:
					seq.append(lft)
				else:
					break
			seq[seq_move_to] = seq[-1]
			node_moved_to_deleted = self._slots[seq[-1] - 1]
			node_moved_to_deleted.left = del_left
			if seq_move_to + 2 != len(seq):
				# we moved left
				parent_of_node_moved_to_deleted = self._slots[seq[-2] - 1]
				parent_of_node_moved_to_deleted.left = node_moved_to_deleted.right
				node_moved_to_deleted.right = del_right
				seq[-1] = parent_of_node_moved_to_deleted.left
			else:
				# we moved right once
				seq[-1] = node_moved_to_deleted.right
		# update parent link
		if seq_move_to > 0:
			par_node = self._slots[seq[seq_move_to - 1] - 1]
			if is_less:
				par_node.left = seq[seq_move_to]
			else:
				par_node.right = seq[seq_move_to]
		else:
			self._root = seq[seq_move_to]
		# patch balance
		if seq[seq_move_to] != 0:
			seq_move_to_node = self._slots[seq[seq_move_to] - 1]
			if special_null:
				seq_move_to_node.balance = i8(0)
			else:
				seq_move_to_node.balance = del_balance

		# rebalance
		while len(seq) >= 2:
			cur = seq[-1]
			par = seq[-2]
			par_node = self._slots[par - 1]
			if special_null:
				is_left = is_less
			else:
				is_left = cur == par_node.left
			special_null = False
			# we inserted to null place, so we increaced it depth
			delta = -(-1 if is_left else 1)
			new_b = par_node.balance + delta
			if new_b == -2:
				gp = 0 if len(seq) == 2 else seq[-3]
				sib = par_node.left
				sib_node = self._slots[sib - 1]
				sib_bal = sib_node.balance
				if sib_bal > 0:
					right_child = sib_node.right
					self._rot_left_right(par, sib, right_child)
					seq.pop()  # cur
					seq.pop()  # par
					seq.append(right_child)
				else:
					self._rot_right(par, sib)
					seq.pop(-2)  # par
					seq[-1] = sib
				if gp != 0:
					gp = self._slots[gp - 1]
					if gp.left == par:
						gp.left = u32(seq[-1])
					else:
						assert gp.right == par
						gp.right = u32(seq[-1])
				if sib_bal == 0:
					break
			elif new_b == 2:
				gp = 0 if len(seq) == 2 else seq[-3]
				sib = par_node.right
				sib_node = self._slots[sib - 1]
				sib_bal = sib_node.balance
				if sib_bal < 0:
					left_child = sib_node.left
					self._rot_right_left(par, sib, left_child)
					seq.pop()  # cur
					seq.pop()  # par
					seq.append(left_child)
				else:
					self._rot_left(par, sib)
					seq.pop(-2)  # par
					seq[-1] = sib
				if gp != 0:
					gp = self._slots[gp - 1]
					if gp.left == par:
						gp.left = u32(seq[-1])
					else:
						assert gp.right == par
						gp.right = u32(seq[-1])
				if sib_bal == 0:
					break
			else:
				par_node.balance = i8(new_b)
				if new_b != 0:
					break
				seq.pop()
		if self._root != seq[0]:
			self._root = seq[0]

	def __setitem__(self, k: K, v: V):
		def setter(node: _Node[K, V]):
			node.value = v

		self._get_set(
			k,
			setter,
			lambda: v,
		)

	def compute_if_absent(self, k: K, supplier: typing.Callable[[], V]) -> V:
		"""
		:returns: Value associated with `k` if it is present, otherwise get's new value from the supplier, stores it at `k` and returns
		"""
		res: list[V] = []

		def existing(node: _Node[K, V]):
			res.append(node.value)

		self._get_set(
			k,
			existing,
			supplier,
		)
		return res[0]

	def get_or_insert_default(self, k: K) -> V:
		return self._get_set(
			k,
			lambda _k: None,
			lambda: _NO_OBJ,  # type: ignore
		)

	def _get_set(
		self,
		k: K,
		exists: typing.Callable[[_Node[K, V]], None],
		does_not_exist: typing.Callable[[], V],
	) -> V:
		seq, is_less = self._find_seq(k)
		# exists
		if seq[-1] != 0:
			slot = self._slots[seq[-1] - 1]
			exists(slot)
			return slot.value
		# patch root
		if len(seq) == 1:
			idx, cur_node = self._alloc_slot()
			self._root = u32(idx + 1)
			cur_node.__init__(k, does_not_exist())
			return cur_node.value
		# alloc new
		new_idx, new_slot = self._alloc_slot()
		if is_less:
			self._slots[seq[-2] - 1].left = u32(new_idx + 1)
		else:
			self._slots[seq[-2] - 1].right = u32(new_idx + 1)
		seq[-1] = new_idx + 1
		new_slot.__init__(k, does_not_exist())
		# rebalance
		while len(seq) >= 2:
			cur = seq[-1]
			par = seq[-2]
			par_node = self._slots[par - 1]
			is_left = cur == par_node.left
			# we inserted to null place, so we increaced it depth
			delta = -1 if is_left else 1
			new_b = par_node.balance + delta
			if new_b == -2:
				gp = 0 if len(seq) == 2 else seq[-3]
				cur_node = self._slots[cur - 1]
				if cur_node.balance > 0:
					right_child = cur_node.right
					self._rot_left_right(par, cur, right_child)
					seq.pop()  # cur
					seq.pop()  # par
					seq.append(right_child)
				else:
					self._rot_right(par, cur)
					seq.pop(-2)  # par
				if gp != 0:
					gp = self._slots[gp - 1]
					if gp.left == par:
						gp.left = u32(seq[-1])
					else:
						gp.right = u32(seq[-1])
				break
			elif new_b == 2:
				gp = 0 if len(seq) == 2 else seq[-3]
				cur_node = self._slots[cur - 1]
				if cur_node.balance < 0:
					left_child = cur_node.left
					self._rot_right_left(par, cur, left_child)
					seq.pop()  # cur
					seq.pop()  # par
					seq.append(left_child)
				else:
					self._rot_left(par, cur)
					seq.pop(-2)  # par
				if gp != 0:
					gp = self._slots[gp - 1]
					if gp.left == par:
						gp.left = u32(seq[-1])
					else:
						gp.right = u32(seq[-1])
				break
			else:
				par_node.balance = i8(new_b)
				if new_b == 0:
					break
				seq.pop()
		if self._root != seq[0]:
			self._root = seq[0]
		return new_slot.value

	def _get_fn[T](
		self,
		k: K,
		found: collections.abc.Callable[[_Node[K, V]], T],
		not_found: collections.abc.Callable[[], T],
	) -> T:
		idx = self._root
		while idx != 0:
			_Node = self._slots[idx - 1]
			if _Node.key == k:
				return found(_Node)
			if k < _Node.key:
				idx = _Node.left
			else:
				idx = _Node.right
		return not_found()

	def get[G](self, k: K, default: G = None) -> V | G:
		"""
		:returns: Value associated with `k` or `default` if there is no such value
		"""
		return self._get_fn(k, lambda n: n.value, lambda: default)

	def __getitem__(self, k: K) -> V:
		def not_found() -> V:
			raise KeyError()

		return self._get_fn(k, lambda x: x.value, not_found)

	def __contains__(self, k: K) -> bool:
		return self._get_fn(k, lambda _: True, lambda: False)

	def _visit[T](
		self, cb: collections.abc.Callable[[_Node[K, V]], T]
	) -> typing.Generator[T, None, None]:
		def go(idx) -> typing.Generator[T, None, None]:
			if idx == 0:
				return
			slot = self._slots[idx - 1]
			yield from go(slot.left)
			yield cb(slot)
			yield from go(slot.right)

		yield from go(self._root)

	def __repr__(self) -> str:
		import json

		ret: list[str] = []
		ret.append('{')
		comma = False
		for k, v in self.items():
			if comma:
				ret.append(',')
			comma = True
			ret.append(json.dumps(k))
			ret.append(':')
			ret.append(repr(v))
		ret.append('}')
		return ''.join(ret)

	def __iter__(self):
		yield from self._visit(lambda n: n.key)

	def items(self) -> collections.abc.ItemsView[K, V]:
		return _ItemsView(self)


class _ItemsView[K: Comparable, V](collections.abc.ItemsView):
	__slots__ = ('_parent',)

	def __init__(self, parent: TreeMap[K, V]):
		self._parent = parent

	def __iter__(self):
		yield from self._parent._visit(lambda n: (n.key, n.value))

	def __contains__(self, item: object) -> bool:
		return any(item == x for x in iter(self))

	def __len__(self):
		return len(self._parent)
