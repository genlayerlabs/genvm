use arbitrary::Arbitrary;
use genlayer_calldata as calldata;
use std::collections::BTreeMap;

fn main() {
    afl::fuzz!(|data: &[u8]| {
        let mut u = arbitrary::Unstructured::new(data);
        let generated = match calldata::Value::arbitrary(&mut u) {
            Ok(m) => m,
            Err(_) => return,
        };

        let encoded = calldata::encode(&generated);
        let decoded = calldata::decode(&encoded).unwrap();

        assert_eq!(generated, decoded);

        let encoded_with_codec = {
            let mut buf = Vec::new();
            calldata::codec::Encode::encode(&generated, &mut calldata::Encoder::new(&mut buf))
                .unwrap();
            buf
        };
        assert_eq!(encoded, encoded_with_codec, "Codec encoding mismatch");
        let decoded_with_codec = calldata::codec::Decode::decode(
            calldata::codec::BinaryDeserializer::new(&encoded_with_codec),
        )
        .unwrap();
        assert_eq!(decoded, decoded_with_codec, "Codec decoding mismatch");
    });
}
