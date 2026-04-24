use std::collections::BTreeMap;

use crate::calldata;

enum Frame {
    Map {
        key_in_parent: String,
        iter: serde_json::map::IntoIter,
        built: BTreeMap<String, calldata::Value>,
    },
    Array {
        key_in_parent: String,
        iter: std::vec::IntoIter<serde_json::Value>,
        built: Vec<calldata::Value>,
    },
}

fn leaf_to_calldata(value: serde_json::Value) -> calldata::Value {
    match value {
        serde_json::Value::Null => calldata::Value::Null,
        serde_json::Value::Bool(b) => calldata::Value::Bool(b),
        serde_json::Value::String(s) => calldata::Value::Str(s),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                calldata::Value::Number(num_bigint::BigInt::from(i))
            } else if let Some(u) = n.as_u64() {
                calldata::Value::Number(num_bigint::BigInt::from(u))
            } else {
                calldata::Value::Str(n.to_string())
            }
        }
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => unreachable!(),
    }
}

fn insert_into_parent(parent: &mut Frame, key: String, value: calldata::Value) {
    match parent {
        Frame::Map { built, .. } => {
            built.insert(key, value);
        }
        Frame::Array { built, .. } => {
            built.push(value);
        }
    }
}

pub fn json_map_to_calldata(
    input: serde_json::Map<String, serde_json::Value>,
) -> BTreeMap<String, calldata::Value> {
    let mut stack: Vec<Frame> = Vec::with_capacity(8);
    stack.push(Frame::Map {
        key_in_parent: String::new(),
        iter: input.into_iter(),
        built: BTreeMap::new(),
    });

    loop {
        let next = match stack.last_mut().expect("stack is non-empty") {
            Frame::Map { iter, .. } => iter.next().map(|(k, v)| (Some(k), v)),
            Frame::Array { iter, .. } => iter.next().map(|v| (None, v)),
        };

        let Some((key_opt, value)) = next else {
            let popped = stack.pop().expect("stack is non-empty");
            let (key_in_parent, finished) = match popped {
                Frame::Map {
                    key_in_parent,
                    built,
                    ..
                } => (key_in_parent, calldata::Value::Map(built)),
                Frame::Array {
                    key_in_parent,
                    built,
                    ..
                } => (key_in_parent, calldata::Value::Array(built)),
            };

            if stack.is_empty() {
                match finished {
                    calldata::Value::Map(m) => return m,
                    _ => unreachable!(),
                }
            }

            insert_into_parent(stack.last_mut().unwrap(), key_in_parent, finished);
            continue;
        };

        let key_in_parent = key_opt.unwrap_or_default();
        match value {
            serde_json::Value::Object(m) => {
                stack.push(Frame::Map {
                    key_in_parent,
                    iter: m.into_iter(),
                    built: BTreeMap::new(),
                });
            }
            serde_json::Value::Array(a) => {
                let len = a.len();
                stack.push(Frame::Array {
                    key_in_parent,
                    iter: a.into_iter(),
                    built: Vec::with_capacity(len),
                });
            }
            leaf => {
                let converted = leaf_to_calldata(leaf);
                insert_into_parent(stack.last_mut().unwrap(), key_in_parent, converted);
            }
        }
    }
}
