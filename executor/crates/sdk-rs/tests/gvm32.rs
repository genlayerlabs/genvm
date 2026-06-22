use genlayer_sdk::gvm32;

#[test]
fn round_trip() {
    let cases = [
        // hex, gvm32 (Crockford Base32, lowercase)
        (
            "ab335240fd942ab8191c5e628cd4ff3903c577bda961fb75df08e0303a00527b",
            "ncsn4g7xjgnbg68wbsh8sn7z741waxxxn5gzpxez13g30eg0a9xg",
        ),
        (
            "47b2d8f260c2d48116044bc43fe3de0f",
            "8ysdhwk0rba825g49f23zryy1w",
        ),
        (
            "1f74d74729abdc08f4f84e8f7f8c808c8ed92ee5",
            "3xtdehs9nfe0hx7r9t7qz340hj7djbq5",
        ),
        (
            "a315ab26a0c4829321730c44a26f4497f7da0631402669caa4e24bdcd9db7c87",
            "mcatp9n0rj1968bk1h2a4vt4jzvxm1hh80k6kjn4w95xspevfj3g",
        ),
        (
            "296a445bfa5e1990af299ec74582468ab7a77e495861691ff79ab21234e514b64fc72b294d7305ecdd0febaa13b1bc1a3f359a711bb93dfb2b82804c64354dab",
            "55n48pztbrcs1bs9kv3mb0j6havtezj9b1gpj7zqkas14d752jv4zhsb556q61fcvm7yqagkp6y1mfsnk9rhqe9xzcnr502ccgtmvar",
        ),
        (
            "99a2da84cec54d17325bcee0a079669c1b15eb7ead32246514b75b97862f1e00",
            "k6hdn16ern6hecjvsvga0yb6kgdhbtvynms28s8mpxdsf1hf3r00",
        ),
    ];

    for (hex, b32) in cases {
        let bytes = hex::decode(hex).unwrap();
        assert_eq!(gvm32::encode(&bytes), b32, "encode {hex}");
        assert_eq!(gvm32::decode(b32).unwrap(), bytes, "decode {b32}");
    }
}

#[test]
fn empty() {
    assert_eq!(gvm32::encode(&[]), "");
    assert_eq!(gvm32::decode("").unwrap(), Vec::<u8>::new());
}

#[test]
fn case_insensitive_and_aliases() {
    let bytes = hex::decode("47b2d8f260c2d48116044bc43fe3de0f").unwrap();
    let canonical = "8ysdhwk0rba825g49f23zryy1w";
    // uppercase decodes the same
    assert_eq!(gvm32::decode(&canonical.to_uppercase()).unwrap(), bytes);
    // hyphens are ignored
    assert_eq!(
        gvm32::decode("8ysd-hwk0-rba8-25g4-9f23-zryy-1w").unwrap(),
        bytes
    );
    // crockford aliases: i/l -> 1, o -> 0
    assert_eq!(gvm32::decode("OoIiLl"), gvm32::decode("001111"));
}

#[test]
fn rejects_invalid() {
    // `u` is not in the alphabet and is not an alias
    assert_eq!(gvm32::decode("u"), None);
    // non-zero trailing padding bits
    assert_eq!(gvm32::decode("01"), None);
}
