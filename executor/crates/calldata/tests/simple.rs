use genlayer_calldata::{Encoder, Value, codec, encode};
#[test]
fn test_date_str() {
    let val = Value::Str("2024-11-26T06:42:42.424242Z".to_string());
    let mut buf = Vec::new();
    codec::Encode::encode(&val, &mut Encoder::new(&mut buf)).unwrap();

    assert_eq!(buf, b"\xdc\x012024-11-26T06:42:42.424242Z");
}
