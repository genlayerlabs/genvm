use genvm_common::calldata;

fn main() {
    afl::fuzz!(|data: &[u8]| {
        let decoded = match calldata::decode(data) {
            Ok(decoded) => decoded,
            Err(_) => return,
        };

        let encoded = calldata::encode(&decoded);

        assert_eq!(data, encoded);
    });
}
