use genlayer_calldata as calldata;

fn main() {
    afl::fuzz!(|data: &[u8]| {
        let decoded = calldata::decode(data);

        let decoded_with_codec: std::result::Result<calldata::Value, _> =
            calldata::codec::Decode::decode(calldata::codec::BinaryDeserializer::new(data));

        if decoded.is_ok() != decoded_with_codec.is_ok() {
            panic!(
                "Codec decoding mismatch\ndecoded = {:?}\ndecoded_with_codec = {:?}\ndata = {:?}",
                decoded, decoded_with_codec, data
            );
        }

        if !decoded.is_ok() {
            return;
        }

        let decoded = decoded.unwrap();
        let decoded_with_codec = decoded_with_codec.unwrap();

        assert_eq!(
            decoded, decoded_with_codec,
            "Value roundtrip mismatch: binary"
        );

        let encoded = calldata::encode(&decoded);

        assert_eq!(data, encoded);

        let mut encoded_with_codec = Vec::new();
        calldata::codec::Encode::encode(
            &decoded_with_codec,
            &mut calldata::Encoder::new(&mut encoded_with_codec),
        )
        .unwrap();
        assert_eq!(
            data, encoded_with_codec,
            "binary roundtrip mismatch from value"
        );
    });
}
