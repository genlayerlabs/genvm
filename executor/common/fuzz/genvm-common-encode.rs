use std::collections::BTreeMap;

use genvm_common::calldata;

fn get_len(r: &mut impl std::io::Read) -> anyhow::Result<usize> {
    let mut top = [0; 1];
    r.read_exact(&mut top)?;

    Ok((top[0] % 10) as usize)
}

fn fetch_str(r: &mut impl std::io::Read) -> anyhow::Result<String> {
    let bytes_len = get_len(r)?;

    let mut ret = vec![0; bytes_len];
    r.read_exact(&mut ret)?;

    Ok(String::from_utf8(ret)?)
}

fn gen_atom(r: &mut impl std::io::Read) -> anyhow::Result<calldata::Value> {
    let mut top = [0; 1];
    r.read_exact(&mut top)?;

    match top[0] % 7 {
        0 => Ok(calldata::Value::Null),
        1 => Ok(calldata::Value::Bool(true)),
        2 => Ok(calldata::Value::Bool(false)),
        3 => {
            let mut data = [0; 20];
            r.read_exact(&mut data)?;

            Ok(calldata::Value::Address(calldata::Address::from(data)))
        }
        4 => {
            let bytes_len = get_len(r)?;
            let mut ret = vec![0; bytes_len];

            r.read_exact(&mut ret)?;

            Ok(calldata::Value::Bytes(ret))
        }
        5 => Ok(calldata::Value::Str(fetch_str(r)?)),
        6 => {
            let bytes_len = get_len(r)?;

            let mut ret = vec![0; bytes_len];
            r.read_exact(&mut ret)?;

            Ok(calldata::Value::Number(
                num_bigint::BigInt::from_signed_bytes_le(&ret),
            ))
        }
        _ => unreachable!(),
    }
}

fn gen_some(depth: usize, r: &mut impl std::io::Read) -> anyhow::Result<calldata::Value> {
    if depth == 0 {
        return gen_atom(r);
    }

    let mut top = [0; 1];
    r.read_exact(&mut top)?;

    match top[0] % 5 {
        0 => gen_atom(r),
        1 | 2 => {
            let len = get_len(r)?;
            let mut ret = Vec::new();

            for _i in 0..len {
                ret.push(gen_some(depth - 1, r)?);
            }

            Ok(calldata::Value::Array(ret))
        }

        3 | 4 => {
            let len = get_len(r)?;
            let mut ret = BTreeMap::new();

            for _i in 0..len {
                let k = fetch_str(r)?;

                ret.insert(k, gen_some(depth - 1, r)?);
            }

            Ok(calldata::Value::Map(ret))
        }

        _ => unreachable!(),
    }
}

fn main() {
    afl::fuzz!(|data: &[u8]| {
        let mut data = data;

        let generated = match gen_some(10, &mut data) {
            Ok(generated) => generated,
            Err(_) => return,
        };

        let encoded = calldata::encode(&generated);
        let decoded = calldata::decode(&encoded).unwrap();

        assert_eq!(generated, decoded);
    });
}
