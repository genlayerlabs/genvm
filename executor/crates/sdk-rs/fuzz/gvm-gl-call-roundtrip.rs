#![allow(deprecated)]

use arbitrary::Arbitrary;
use genlayer_calldata::{Encoder, codec};
use genlayer_sdk::abi::gl_call::Message;

fn main() {
    afl::fuzz!(|data: &[u8]| {
        let mut u = arbitrary::Unstructured::new(data);
        let msg = match Message::arbitrary(&mut u) {
            Ok(m) => m,
            Err(_) => return,
        };

        // Encode via calldata codec
        let mut buf = Vec::new();
        codec::Encode::encode(&msg, &mut Encoder::new(&mut buf)).unwrap();

        // Decode binary back to Value
        let decoded_value = genlayer_calldata::decode(&buf).unwrap();

        // Re-encode the decoded Value to binary
        let reencoded = genlayer_calldata::encode(&decoded_value);

        // Binary roundtrip must be identical
        assert_eq!(buf, reencoded, "binary roundtrip mismatch");

        // Decode back to Message via calldata codec
        let msg_decoded: Message =
            codec::Decode::decode(codec::ValueDeserializer(decoded_value)).unwrap();

        assert_eq!(msg, msg_decoded, "Message roundtrip mismatch: value");

        let msg_decoded_from_binary: Message =
            codec::Decode::decode(codec::BinaryDeserializer::new(&buf)).unwrap();

        assert_eq!(
            msg, msg_decoded_from_binary,
            "Message roundtrip mismatch: binary"
        );
    });
}
