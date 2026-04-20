use arbitrary::Arbitrary;
use genlayer_sdk::storage::{Slot, StorageType, TreeMap};
use std::cell::RefCell;
use std::collections::BTreeMap;

// Global paged storage backing the SDK's __genvm_storage_read / __genvm_storage_write.

thread_local! {
    static STORAGE: RefCell<BTreeMap<[u8; 32], Vec<u8>>> = RefCell::new(BTreeMap::new());
}

fn storage_clear() {
    STORAGE.with(|s| s.borrow_mut().clear());
}

#[unsafe(no_mangle)]
pub fn __genvm_storage_read(slot: &[u8; 32], offset: u32, buf: &mut [u8]) -> u32 {
    STORAGE.with(|s| {
        let s = s.borrow();
        if let Some(data) = s.get(slot) {
            let off = offset as usize;
            for (i, b) in buf.iter_mut().enumerate() {
                *b = data.get(off + i).copied().unwrap_or(0);
            }
        } else {
            buf.fill(0);
        }
    });
    0
}

#[unsafe(no_mangle)]
pub fn __genvm_storage_write(slot: &[u8; 32], offset: u32, buf: &[u8]) -> u32 {
    STORAGE.with(|s| {
        let mut s = s.borrow_mut();
        let data = s.entry(*slot).or_default();
        let end = offset as usize + buf.len();
        if data.len() < end {
            data.resize(end, 0);
        }
        data[offset as usize..end].copy_from_slice(buf);
    });
    0
}

// Fuzz operations

#[derive(Debug, Arbitrary)]
enum Op {
    Insert { key: u32, value: u32 },
    Remove { key: u32 },
    Get { key: u32 },
}

fn check_order(tree: &<TreeMap<u32, u32> as StorageType>::Handle, reference: &BTreeMap<u32, u32>) {
    let mut entries = Vec::new();
    tree.for_each(|k, v| entries.push((k, v)));
    let ref_entries: Vec<(u32, u32)> = reference.iter().map(|(&k, &v)| (k, v)).collect();
    assert_eq!(
        entries, ref_entries,
        "order mismatch:\n  tree:      {entries:?}\n  reference: {ref_entries:?}"
    );
}

fn run_fuzz(ops: Vec<Op>) {
    storage_clear();

    let slot = Slot([0u8; 32]);
    let tree = <TreeMap<u32, u32> as StorageType>::handle_at(slot, 0);
    let mut reference = BTreeMap::<u32, u32>::new();

    for op in ops {
        match op {
            Op::Insert { key, value } => {
                eprintln!("insert({key}, {value})");
                tree.insert(&key, &value);
                reference.insert(key, value);
                check_order(&tree, &reference);
            }
            Op::Remove { key } => {
                eprintln!("remove({key})");
                let existed = tree.remove(&key);
                let ref_existed = reference.remove(&key).is_some();
                assert_eq!(
                    existed, ref_existed,
                    "remove({key}): tree={existed}, reference={ref_existed}"
                );
                check_order(&tree, &reference);
            }
            Op::Get { key } => {
                eprintln!("get({key})");
                let tree_val = tree.get(&key).map(|h| h.get());
                let ref_val = reference.get(&key).copied();
                assert_eq!(
                    tree_val, ref_val,
                    "get({key}): tree={tree_val:?}, reference={ref_val:?}"
                );
            }
        }
    }
}

fn main() {
    afl::fuzz!(|data: Vec<Op>| {
        run_fuzz(data);
    });
}
