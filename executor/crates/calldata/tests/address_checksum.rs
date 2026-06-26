use genlayer_calldata::Address;

fn addr(lower_hex: &str) -> Address {
    let mut b = [0u8; 20];
    hex::decode_to_slice(lower_hex, &mut b).unwrap();
    Address::from(b)
}

#[test]
fn eip55_checksum() {
    // vectors from EIP-55 (https://eips.ethereum.org/EIPS/eip-55)
    for expected in [
        "5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed",
        "fB6916095ca1df60bB79Ce92cE3Ea74c37c5d359",
        "dbF03B407c01E7cD3CBea99509d93f8DDDC8C6FB",
        "D1220A0cf47c7B9Be7A2E6BA89F429762e7b9aDb",
    ] {
        let a = addr(&expected.to_lowercase());
        assert_eq!(a.checksum_hex().as_slice(), expected.as_bytes());
        assert_eq!(a.checksum_hex_string(), expected);
    }
}
