#[test]
fn test_call_key() {
    use genlayer_sdk::abi::CallKey;

    let key1 = CallKey::for_method("foo");
    let key2 = CallKey::for_method("foo");
    let key3 = CallKey::for_method("bar");

    assert_eq!(key1, key2);
    assert_ne!(key1, key3);
}

#[test]
fn test_call_key_unnamed() {
    use genlayer_sdk::abi::CallKey;

    let key1 = CallKey::for_method("");

    assert_eq!(key1, CallKey::UNNAMED);
}
