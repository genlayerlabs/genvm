use genvm::calldata::Address;
use genvm::public_abi::StorageType;
use genvm::runners::Id;
use genvm::SlotID;

#[test]
fn chain_canonical_uses_checksum_address() {
    // EIP-55 address whose checksum form has mixed case
    let address = Address::from([
        0x5a, 0xae, 0xb6, 0x05, 0x3f, 0x3e, 0x94, 0xc9, 0xb9, 0xa0, 0x9f, 0x33, 0x66, 0x94, 0x35,
        0xe7, 0xef, 0x1b, 0xea, 0xed,
    ]);

    let id = Id::Chain {
        address,
        on: StorageType::LatestNonFinal,
        slot: SlotID::ZERO,
    };

    let expected = format!(
        "chain:0x{}:a:{}",
        std::str::from_utf8(&address.checksum_hex()).unwrap(),
        "0".repeat(52), // gvm32 of 32 zero bytes
    );
    assert_eq!(id.canonical().as_str(), expected.as_str());
    // sanity: the address part is genuinely mixed-case (checksummed), not lowercase
    assert!(expected.contains("5aAeb6053F3E94C9b9A09f33669435E7Ef1BeAed"));
}
