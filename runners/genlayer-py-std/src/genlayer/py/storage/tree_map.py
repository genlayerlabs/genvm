import abc
import collections.abc

import typing
import collections.abc

from .vec import Vec
from genlayer.py.types import u32, i8

class _Node[K, V]:
	key: K
	value: V
	left: u32
	right: u32
	balance: i8

	def __init__(self, k: K, v: V):
		self.key = k
		self.value = v
		self.left = u32(0)
		self.right = u32(0)
		self.balance = i8(0)

class Comparable(typing.Protocol):
	@abc.abstractmethod
	def __eq__(self, other: typing.Any, /) -> bool:
		pass

	@abc.abstractmethod
	def __lt__(self, other: typing.Any, /) -> bool:
		pass

class TreeMap[K: Comparable, V]:
	root: u32
	slots: Vec[_Node[K, V]]
	free_slots: Vec[u32]

	def _alloc_slot(self) -> tuple[int, _Node[K, V]]:
		if len(self.free_slots) > 0:
			idx = int(self.free_slots[-1])
			self.free_slots.pop()
			slot = self.slots[idx]
		else:
			idx = len(self.slots)
			slot = self.slots.append_new_get()
		return (idx, slot)
	def _free_slot(self, slot: u32):
		if slot + 1 == len(self.slots):
			self.slots.pop()
		else:
			self.free_slots.append(slot)

	def _rot_left(self, par: int, cur: int):
		par_node = self.slots[par - 1]
		cur_node = self.slots[cur - 1]
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
		par_node = self.slots[par - 1]
		cur_node = self.slots[cur - 1]
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
		gpar_node = self.slots[gpar - 1]
		par_node = self.slots[par - 1]
		cur_node = self.slots[cur - 1]
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
		gpar_node = self.slots[gpar - 1]
		par_node = self.slots[par - 1]
		cur_node = self.slots[cur - 1]
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
		cur = self.root
		is_less = True
		while True:
			seq.append(cur)
			if cur == 0:
				break
			cur_node = self.slots[cur - 1]
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
		del_node = self.slots[seq[-1] - 1]
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
				cur_node = self.slots[seq[-1] - 1]
				lft = cur_node.left
				if lft != 0:
					seq.append(lft)
				else:
					break
			seq[seq_move_to] = seq[-1]
			node_moved_to_deleted = self.slots[seq[-1] - 1]
			node_moved_to_deleted.left = del_left
			if seq_move_to + 2 != len(seq):
				# we moved left
				parent_of_node_moved_to_deleted = self.slots[seq[-2] - 1]
				parent_of_node_moved_to_deleted.left = node_moved_to_deleted.right
				node_moved_to_deleted.right = del_right
				seq[-1] = parent_of_node_moved_to_deleted.left
			else:
				# we moved right once
				seq[-1] = node_moved_to_deleted.right
		# update parent link
		if seq_move_to > 0:
			par_node = self.slots[seq[seq_move_to - 1] - 1]
			if is_less:
				par_node.left = seq[seq_move_to]
			else:
				par_node.right = seq[seq_move_to]
		else:
			self.root = seq[seq_move_to]
		# patch balance
		if seq[seq_move_to] != 0:
			seq_move_to_node = self.slots[seq[seq_move_to] - 1]
			if special_null:
				seq_move_to_node.balance = i8(0)
			else:
				seq_move_to_node.balance = del_balance

		# rebalance
		while len(seq) >= 2:
			cur = seq[-1]
			par = seq[-2]
			par_node = self.slots[par - 1]
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
				sib_node = self.slots[sib - 1]
				sib_bal = sib_node.balance
				if sib_bal > 0:
					right_child = sib_node.right
					self._rot_left_right(par, sib, right_child)
					seq.pop() # cur
					seq.pop() # par
					seq.append(right_child)
				else:
					self._rot_right(par, sib)
					seq.pop(-2) # par
					seq[-1] = sib
				if gp != 0:
					gp = self.slots[gp - 1]
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
				sib_node = self.slots[sib - 1]
				sib_bal = sib_node.balance
				if sib_bal < 0:
					left_child = sib_node.left
					self._rot_right_left(par, sib, left_child)
					seq.pop() # cur
					seq.pop() # par
					seq.append(left_child)
				else:
					self._rot_left(par, sib)
					seq.pop(-2) # par
					seq[-1] = sib
				if gp != 0:
					gp = self.slots[gp - 1]
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
		if self.root != seq[0]:
			self.root = seq[0]

	def __setitem__(self, k: K, v: V):
		seq, is_less = self._find_seq(k)
		# replace existing
		if seq[-1] != 0:
			self.slots[seq[-1] - 1].value = v
			return
		# patch root
		if len(seq) == 1:
			idx, cur_node = self._alloc_slot()
			self.root = u32(idx + 1)
			cur_node.__init__(k, v)
			return
		# alloc new
		new_idx, new_slot = self._alloc_slot()
		if is_less:
			self.slots[seq[-2] - 1].left = u32(new_idx + 1)
		else:
			self.slots[seq[-2] - 1].right = u32(new_idx + 1)
		seq[-1] = new_idx + 1
		new_slot.__init__(k, v)
		# rebalance
		while len(seq) >= 2:
			cur = seq[-1]
			par = seq[-2]
			par_node = self.slots[par - 1]
			is_left = cur == par_node.left
			# we inserted to null place, so we increaced it depth
			delta = (-1 if is_left else 1)
			new_b = par_node.balance + delta
			if new_b == -2:
				gp = 0 if len(seq) == 2 else seq[-3]
				cur_node = self.slots[cur - 1]
				if cur_node.balance > 0:
					right_child = cur_node.right
					self._rot_left_right(par, cur, right_child)
					seq.pop() # cur
					seq.pop() # par
					seq.append(right_child)
				else:
					self._rot_right(par, cur)
					seq.pop(-2) # par
				if gp != 0:
					gp = self.slots[gp - 1]
					if gp.left == par:
						gp.left = u32(seq[-1])
					else:
						gp.right = u32(seq[-1])
				break
			elif new_b == 2:
				gp = 0 if len(seq) == 2 else seq[-3]
				cur_node = self.slots[cur - 1]
				if cur_node.balance < 0:
					left_child = cur_node.left
					self._rot_right_left(par, cur, left_child)
					seq.pop() # cur
					seq.pop() # par
					seq.append(left_child)
				else:
					self._rot_left(par, cur)
					seq.pop(-2) # par
				if gp != 0:
					gp = self.slots[gp - 1]
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
		if self.root != seq[0]:
			self.root = seq[0]

	def get_fn[T](self, k: K, found: collections.abc.Callable[[_Node[K, V]], T], not_found: collections.abc.Callable[[], T]) -> T:
		idx = self.root
		while idx != 0:
			_Node = self.slots[idx - 1]
			if _Node.key == k:
				return found(_Node)
			if k < _Node.key:
				idx = _Node.left
			else:
				idx = _Node.right
		return not_found()

	def get(self, k: K, dflt=None): #  -> V | None
		return self.get_fn(k, lambda n: n.value, lambda: dflt)

	def __getitem__(self, k: K) -> V:
		def not_found() -> V:
			raise KeyError()
		return self.get_fn(k, lambda x: x.value, not_found)

	def __contains__(self, k: K) -> bool:
		return self.get_fn(k, lambda _: True, lambda: False)

	def visit[T](self, cb: collections.abc.Callable[[_Node[K, V]], T]):
		def go(idx):
			if idx == 0:
				return
			slot = self.slots[idx - 1]
			yield from go(slot.left)
			yield cb(slot)
			yield from go(slot.right)
		yield from go(self.root)

	def __iter__(self):
		yield from self.visit(lambda n: n.key)

	def items(self):
		yield from self.visit(lambda n: (n.key, n.value))
