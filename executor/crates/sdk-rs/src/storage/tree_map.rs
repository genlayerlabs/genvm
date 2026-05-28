//! AVL tree map backed by contract storage.
//!
//! Port of `genlayer.storage.tree_map.TreeMap`.

use super::core::{StorageType, StorageValue};

// Node: key, value, left child index (1-based), right child index (1-based), AVL balance factor
crate::record!(TreeMapNode[K, V] {
    key: K,
    value: V,
    left: u32,
    right: u32,
    balance: i8,
});

// TreeMap layout: root index (1-based, 0=empty), node pool, free-slot stack
crate::record!(TreeMap[K, V] {
    root_idx: u32,
    slots: super::array::DynArray<TreeMapNode<K, V>>,
    free_slots: super::array::DynArray<u32>,
});

impl<K, V> TreeMap<K, V>
where
    K: StorageValue,
    K::Value: Ord,
    V: StorageType,
{
    // ---- helpers ----

    /// Get node handle by 1-based index.
    fn node(&self, idx: u32) -> TreeMapNode<K, V> {
        self.slots().index(idx - 1)
    }

    /// Walk from root to `k`, returning the path (1-based indices, last is 0 if
    /// not found) and whether the last comparison was "go left".
    fn find_path(&self, k: &K::Value) -> (Vec<u32>, bool) {
        let mut path = Vec::new();
        let mut cur = self.root_idx().get();
        let mut is_left = true;
        loop {
            path.push(cur);
            if cur == 0 {
                break;
            }
            let node = self.node(cur);
            let node_key = K::storage_get(&node.key());
            if *k < node_key {
                cur = node.left().get();
                is_left = true;
            } else if node_key < *k {
                cur = node.right().get();
                is_left = false;
            } else {
                break; // found
            }
        }
        (path, is_left)
    }

    /// Allocate a slot (recycle or append). Returns (0-based index, node handle).
    fn alloc_slot(&self) -> (u32, TreeMapNode<K, V>) {
        let free = self.free_slots();
        if !free.is_empty() {
            let idx = free.index(free.len() - 1).get();
            free.pop();
            (idx, self.slots().index(idx))
        } else {
            let idx = self.slots().len();
            let node = self.slots().append_slot();
            (idx, node)
        }
    }

    /// Return a slot to the free list (or shrink pool).
    fn free_slot(&self, slot_idx: u32) {
        if slot_idx + 1 == self.slots().len() {
            self.slots().pop();
        } else {
            self.free_slots().append_slot().set(slot_idx);
        }
    }

    fn init_node(&self, node: TreeMapNode<K, V>, k: &K::Value)
    where
        K: StorageValue,
    {
        K::storage_set(&node.key(), k);
        node.left().set(0);
        node.right().set(0);
        node.balance().set(0);
    }

    // ---- rotations ----

    fn rot_left(&self, par: u32, cur: u32) {
        let par_n = self.node(par);
        let cur_n = self.node(cur);
        let cur_l = cur_n.left().get();
        cur_n.left().set(par);
        par_n.right().set(cur_l);
        if cur_n.balance().get() == 0 {
            par_n.balance().set(1);
            cur_n.balance().set(-1);
        } else {
            par_n.balance().set(0);
            cur_n.balance().set(0);
        }
    }

    fn rot_right(&self, par: u32, cur: u32) {
        let par_n = self.node(par);
        let cur_n = self.node(cur);
        let cur_r = cur_n.right().get();
        cur_n.right().set(par);
        par_n.left().set(cur_r);
        if cur_n.balance().get() == 0 {
            par_n.balance().set(-1);
            cur_n.balance().set(1);
        } else {
            par_n.balance().set(0);
            cur_n.balance().set(0);
        }
    }

    fn rot_right_left(&self, gpar: u32, par: u32, cur: u32) {
        let gpar_n = self.node(gpar);
        let par_n = self.node(par);
        let cur_n = self.node(cur);
        let cur_l = cur_n.left().get();
        let cur_r = cur_n.right().get();

        gpar_n.right().set(cur_l);
        par_n.left().set(cur_r);
        cur_n.left().set(gpar);
        cur_n.right().set(par);

        let bal = cur_n.balance().get();
        if bal == 0 {
            par_n.balance().set(0);
            gpar_n.balance().set(0);
        } else if bal > 0 {
            gpar_n.balance().set(-1);
            par_n.balance().set(0);
        } else {
            gpar_n.balance().set(0);
            par_n.balance().set(1);
        }
        cur_n.balance().set(0);
    }

    fn rot_left_right(&self, gpar: u32, par: u32, cur: u32) {
        let gpar_n = self.node(gpar);
        let par_n = self.node(par);
        let cur_n = self.node(cur);
        let cur_l = cur_n.left().get();
        let cur_r = cur_n.right().get();

        gpar_n.left().set(cur_r);
        par_n.right().set(cur_l);
        cur_n.left().set(par);
        cur_n.right().set(gpar);

        let bal = cur_n.balance().get();
        if bal == 0 {
            par_n.balance().set(0);
            gpar_n.balance().set(0);
        } else if bal > 0 {
            par_n.balance().set(-1);
            gpar_n.balance().set(0);
        } else {
            par_n.balance().set(0);
            gpar_n.balance().set(1);
        }
        cur_n.balance().set(0);
    }

    /// Update parent link or root after a rotation replaces `old_child` with `new_child`.
    fn replace_child(&self, seq: &[u32], depth: usize, old_child: u32, new_child: u32) {
        if depth == 0 {
            self.root_idx().set(new_child);
        } else {
            let gp = self.node(seq[depth - 1]);
            if gp.left().get() == old_child {
                gp.left().set(new_child);
            } else {
                gp.right().set(new_child);
            }
        }
    }

    // ---- public API ----

    pub fn len(&self) -> u32 {
        self.slots().len() - self.free_slots().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn clear(&self) {
        self.root_idx().set(0);
        self.slots().clear();
        self.free_slots().clear();
    }

    pub fn contains_key(&self, k: &K::Value) -> bool {
        let (path, _) = self.find_path(k);
        *path.last().unwrap() != 0
    }

    /// Look up a key and return a handle to its value, or `None`.
    pub fn get(&self, k: &K::Value) -> Option<<V as StorageType>::Handle> {
        let (path, _) = self.find_path(k);
        let last = *path.last().unwrap();
        if last == 0 {
            None
        } else {
            Some(self.node(last).value())
        }
    }

    /// Insert or update. Returns a handle to the value slot.
    pub fn insert(&self, k: &K::Value, v: &<V as StorageValue>::Value) -> <V as StorageType>::Handle
    where
        V: StorageValue,
    {
        let (mut seq, is_left) = self.find_path(k);

        // key exists → update value
        if *seq.last().unwrap() != 0 {
            let node = self.node(*seq.last().unwrap());
            V::storage_set(&node.value(), v);
            return node.value();
        }

        // empty tree
        if seq.len() == 1 {
            let (idx, node) = self.alloc_slot();
            self.root_idx().set(idx + 1);
            self.init_node(node, k);
            V::storage_set(&node.value(), v);
            return node.value();
        }

        // allocate new node and link to parent
        let (new_idx, new_node) = self.alloc_slot();
        let parent = self.node(seq[seq.len() - 2]);
        if is_left {
            parent.left().set(new_idx + 1);
        } else {
            parent.right().set(new_idx + 1);
        }
        let last = seq.len() - 1;
        seq[last] = new_idx + 1;
        self.init_node(new_node, k);
        V::storage_set(&new_node.value(), v);
        let ret = new_node.value();

        // rebalance after insert
        while seq.len() >= 2 {
            let cur = *seq.last().unwrap();
            let par = seq[seq.len() - 2];
            let par_n = self.node(par);
            let is_left = cur == par_n.left().get();
            let delta: i8 = if is_left { -1 } else { 1 };
            let new_b = par_n.balance().get() + delta;

            if new_b == -2 {
                let gp_depth = seq.len() - 2;
                let cur_n = self.node(cur);
                if cur_n.balance().get() > 0 {
                    let right_child = cur_n.right().get();
                    self.rot_left_right(par, cur, right_child);
                    seq.pop(); // cur
                    seq.pop(); // par
                    seq.push(right_child);
                } else {
                    self.rot_right(par, cur);
                    let slen = seq.len();
                    seq.remove(slen - 2); // par
                }
                self.replace_child(&seq, gp_depth, par, *seq.last().unwrap());
                break;
            } else if new_b == 2 {
                let gp_depth = seq.len() - 2;
                let cur_n = self.node(cur);
                if cur_n.balance().get() < 0 {
                    let left_child = cur_n.left().get();
                    self.rot_right_left(par, cur, left_child);
                    seq.pop(); // cur
                    seq.pop(); // par
                    seq.push(left_child);
                } else {
                    self.rot_left(par, cur);
                    let slen = seq.len();
                    seq.remove(slen - 2); // par
                }
                self.replace_child(&seq, gp_depth, par, *seq.last().unwrap());
                break;
            } else {
                par_n.balance().set(new_b);
                if new_b == 0 {
                    break;
                }
                seq.pop();
            }
        }
        if self.root_idx().get() != seq[0] {
            self.root_idx().set(seq[0]);
        }
        ret
    }

    /// Remove the entry with the given key. Returns `true` if it was present.
    pub fn remove(&self, k: &K::Value) -> bool {
        let (mut seq, is_left) = self.find_path(k);

        // not found
        if *seq.last().unwrap() == 0 {
            return false;
        }

        let del_idx = *seq.last().unwrap();
        let del_node = self.node(del_idx);
        let del_left = del_node.left().get();
        let del_right = del_node.right().get();
        let del_balance = del_node.balance().get();
        self.free_slot(del_idx - 1);

        let mut special_null = false;
        let seq_move_to = seq.len() - 1;

        if del_left == 0 || del_right == 0 {
            // <= 1 child
            seq[seq_move_to] = if del_left == 0 { del_right } else { del_left };
            special_null = true;
        } else {
            // two children: find in-order successor (go right, then left*)
            seq.push(del_right);
            loop {
                let cur_n = self.node(*seq.last().unwrap());
                let lft = cur_n.left().get();
                if lft != 0 {
                    seq.push(lft);
                } else {
                    break;
                }
            }
            let replacement = *seq.last().unwrap();
            seq[seq_move_to] = replacement;
            let rep_n = self.node(replacement);
            rep_n.left().set(del_left);
            if seq_move_to + 2 != seq.len() {
                // moved left: detach replacement from its parent
                let parent_of_rep = self.node(seq[seq.len() - 2]);
                parent_of_rep.left().set(rep_n.right().get());
                rep_n.right().set(del_right);
                let slen = seq.len();
                seq[slen - 1] = self.node(seq[seq.len() - 2]).left().get();
            } else {
                // moved right once
                let slen = seq.len();
                seq[slen - 1] = rep_n.right().get();
            }
        }

        // update parent link
        if seq_move_to > 0 {
            let par_n = self.node(seq[seq_move_to - 1]);
            if is_left {
                par_n.left().set(seq[seq_move_to]);
            } else {
                par_n.right().set(seq[seq_move_to]);
            }
        } else {
            self.root_idx().set(seq[seq_move_to]);
        }

        // patch balance of replacement
        if seq[seq_move_to] != 0 {
            let rep_n = self.node(seq[seq_move_to]);
            if special_null {
                rep_n.balance().set(0);
            } else {
                rep_n.balance().set(del_balance);
            }
        }

        // rebalance after delete
        while seq.len() >= 2 {
            let cur = *seq.last().unwrap();
            let par = seq[seq.len() - 2];
            let par_n = self.node(par);
            let is_left_child = if special_null {
                is_left
            } else {
                cur == par_n.left().get()
            };
            special_null = false;
            // deletion decreased depth on the is_left_child side
            let delta: i8 = if is_left_child { 1 } else { -1 };
            let new_b = par_n.balance().get() + delta;

            if new_b == -2 {
                let gp_depth = seq.len() - 2;
                let sib = par_n.left().get();
                let sib_n = self.node(sib);
                let sib_bal = sib_n.balance().get();
                if sib_bal > 0 {
                    let right_child = sib_n.right().get();
                    self.rot_left_right(par, sib, right_child);
                    seq.pop();
                    seq.pop();
                    seq.push(right_child);
                } else {
                    self.rot_right(par, sib);
                    let slen = seq.len();
                    seq.remove(slen - 2);
                    let slen = seq.len();
                    seq[slen - 1] = sib;
                }
                self.replace_child(&seq, gp_depth, par, *seq.last().unwrap());
                if sib_bal == 0 {
                    break;
                }
            } else if new_b == 2 {
                let gp_depth = seq.len() - 2;
                let sib = par_n.right().get();
                let sib_n = self.node(sib);
                let sib_bal = sib_n.balance().get();
                if sib_bal < 0 {
                    let left_child = sib_n.left().get();
                    self.rot_right_left(par, sib, left_child);
                    seq.pop();
                    seq.pop();
                    seq.push(left_child);
                } else {
                    self.rot_left(par, sib);
                    let slen = seq.len();
                    seq.remove(slen - 2);
                    let slen = seq.len();
                    seq[slen - 1] = sib;
                }
                self.replace_child(&seq, gp_depth, par, *seq.last().unwrap());
                if sib_bal == 0 {
                    break;
                }
            } else {
                par_n.balance().set(new_b);
                if new_b != 0 {
                    break;
                }
                seq.pop();
            }
        }
        if self.root_idx().get() != seq[0] {
            self.root_idx().set(seq[0]);
        }
        true
    }

    // ---- iteration ----

    fn visit_impl<F>(&self, idx: u32, f: &mut F)
    where
        F: FnMut(TreeMapNode<K, V>),
    {
        if idx == 0 {
            return;
        }
        let node = self.node(idx);
        self.visit_impl(node.left().get(), f);
        f(node);
        self.visit_impl(node.right().get(), f);
    }

    /// Iterate entries in key order, calling `f` with each node handle.
    pub fn for_each_node<F>(&self, mut f: F)
    where
        F: FnMut(TreeMapNode<K, V>),
    {
        self.visit_impl(self.root_idx().get(), &mut f);
    }

    /// Iterate entries in key order, calling `f(key, value)`.
    pub fn for_each<F>(&self, mut f: F)
    where
        V: StorageValue,
        F: FnMut(K::Value, <V as StorageValue>::Value),
    {
        self.for_each_node(|node| {
            let k = K::storage_get(&node.key());
            let v = V::storage_get(&node.value());
            f(k, v);
        });
    }
}
