use genlayer_sdk::abi::gl_call::On;
use genvm_common::domain::fees::{
    ExternalMessageParams, InternalMessageParams, MessageAllocationNode,
    MessageAllocationNodeParams,
};
use primitive_types::U256;

/// Concatenates 32-byte big-endian ABI words into one buffer.
fn words(values: &[U256]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(values.len() * 32);
    for v in values {
        buf.extend_from_slice(&v.to_big_endian());
    }
    buf
}

/// Reads the `i`-th 32-byte ABI word from an encoded buffer.
fn word(buf: &[u8], i: usize) -> U256 {
    U256::from_big_endian(&buf[i * 32..i * 32 + 32])
}

fn external_node(
    recipient: Option<[u8; 20]>,
    call_key: Option<[u8; 32]>,
    budget: u64,
    gas_limit: u64,
    max_gas_price: u64,
    children: Vec<MessageAllocationNode>,
) -> MessageAllocationNode {
    MessageAllocationNode {
        recipient: recipient.map(genlayer_sdk::calldata::Address::from),
        call_key: call_key.map(genlayer_sdk::abi::CallKey),
        budget: U256::from(budget),
        // External messages have no acceptance/finalize lifecycle; value is unused.
        on: On::Finalized,
        fee_params: MessageAllocationNodeParams::External(ExternalMessageParams {
            gas_limit: U256::from(gas_limit),
            max_gas_price: U256::from(max_gas_price),
        }),
        children,
    }
}

fn internal_node(
    on: On,
    budget: u64,
    rotations: &[u64],
    children: Vec<MessageAllocationNode>,
) -> MessageAllocationNode {
    MessageAllocationNode {
        recipient: None,
        call_key: None,
        budget: U256::from(budget),
        on,
        fee_params: MessageAllocationNodeParams::Internal(std::sync::Arc::new(
            InternalMessageParams {
                leader_timeunits_allocation: U256::from(1),
                validator_timeunits_allocation: U256::from(2),
                execution_budget_per_round: U256::from(3),
                rotations: rotations.iter().map(|r| U256::from(*r)).collect(),
                max_price_gen_per_time_unit: U256::from(11),
                storage_fee_max_gas_price: U256::from(12),
                receipt_fee_max_gas_price: U256::from(13),
            },
        )),
        children,
    }
}

// ── Exact wire layout ───────────────────────────────────────────────

#[test]
fn external_root_node_matches_exact_encoding() {
    let recipient = [0x11u8; 20];
    let encoded =
        MessageAllocationNode::abi_encode(&[external_node(Some(recipient), None, 5, 7, 9, vec![])]);

    // `abi.encode(MessageAllocationNode[])` of a single external root node:
    // array offset, length, element offset, then the 10-word element tuple
    // (messageType=External, onAcceptance=false, parent=sentinel, recipient,
    // callKey wildcard, budget, feeParams offset, feeParams len, gasLimit, maxGasPrice).
    let expected = words(&[
        U256::from(0x20),                  // offset to array
        U256::from(1),                     // array length
        U256::from(0x20),                  // element[0] head offset
        U256::from(0),                     // messageType = External
        U256::from(0),                     // onAcceptance = false
        U256::MAX,                         // parentIndex = NODE_ROOT_SENTINEL
        U256::from_big_endian(&recipient), // recipient (left-padded)
        U256::from(0),                     // callKey = CALL_KEY_WILDCARD
        U256::from(5),                     // budget
        U256::from(0xE0),                  // feeParams offset (7 head words)
        U256::from(64),                    // feeParams bytes length
        U256::from(7),                     // gasLimit
        U256::from(9),                     // maxGasPrice
    ]);

    assert_eq!(encoded, expected);
}

// ── Tree flattening to parent-pointer form ──────────────────────────

#[test]
fn nested_internal_flattens_with_parent_pointers() {
    // root (internal, accepted) with a single external child.
    let child = external_node(Some([0x22u8; 20]), None, 1, 100, 200, vec![]);
    let root = internal_node(On::Accepted, 10, &[2, 3], vec![child]);

    let encoded = MessageAllocationNode::abi_encode(&[root]);

    assert_eq!(word(&encoded, 0), U256::from(0x20));
    assert_eq!(word(&encoded, 1), U256::from(2), "two flattened nodes");

    // Heads region begins right after the length word (word index 2), and the
    // per-element offsets there are relative to it.
    let heads_base = 2 * 32;
    let root_idx = (heads_base + word(&encoded, 2).as_usize()) / 32;
    let child_idx = (heads_base + word(&encoded, 3).as_usize()) / 32;

    // Root: messageType Internal (1), onAcceptance true, parent = sentinel.
    assert_eq!(
        word(&encoded, root_idx),
        U256::from(1),
        "root messageType Internal"
    );
    assert_eq!(
        word(&encoded, root_idx + 1),
        U256::one(),
        "root onAcceptance"
    );
    assert_eq!(
        word(&encoded, root_idx + 2),
        U256::MAX,
        "root parent = sentinel"
    );

    // Child: messageType External (0), parent index = 0 (root is first flattened node).
    assert_eq!(
        word(&encoded, child_idx),
        U256::from(0),
        "child messageType External"
    );
    assert_eq!(
        word(&encoded, child_idx + 1),
        U256::zero(),
        "child onAcceptance false"
    );
    assert_eq!(
        word(&encoded, child_idx + 2),
        U256::zero(),
        "child parent index 0"
    );
}

// ── Chain-derived feeParams fields ──────────────────────────────────

#[test]
fn internal_params_encode_derived_appeal_rounds() {
    // appealRounds is not stored on the Rust side; it is reconstructed as
    // len(rotations) - 1 when encoding.
    let encoded =
        MessageAllocationNode::abi_encode(&[internal_node(On::Finalized, 10, &[2, 3, 4], vec![])]);

    // Walk to the feeParams bytes inside the single element.
    let heads_base = 2 * 32;
    let elem_idx = (heads_base + word(&encoded, 2).as_usize()) / 32;
    // element head is 7 words; the feeParams offset word is the 7th (elem_idx + 6).
    let fee_params_len_idx = elem_idx + word(&encoded, elem_idx + 6).as_usize() / 32;
    // feeParams = [len][ abi.encode(InternalMessageParams) ], tuple starts after len.
    let inner_base = fee_params_len_idx + 1;

    // InternalMessageParams tuple (v0.6-dev, 8 head words):
    // [offset][leader][validator][appealRounds][exec][rot off][maxPriceGen]
    // [storageCap][receiptCap][rot len][rot...]
    assert_eq!(
        word(&encoded, inner_base),
        U256::from(0x20),
        "struct offset"
    );
    assert_eq!(word(&encoded, inner_base + 1), U256::from(1), "leader");
    assert_eq!(word(&encoded, inner_base + 2), U256::from(2), "validator");
    assert_eq!(
        word(&encoded, inner_base + 3),
        U256::from(2),
        "appealRounds = 3 - 1"
    );
    assert_eq!(
        word(&encoded, inner_base + 4),
        U256::from(3),
        "execBudgetPerRound"
    );
    assert_eq!(
        word(&encoded, inner_base + 5),
        U256::from(0x100),
        "rotations offset (8 head words)"
    );
    assert_eq!(
        word(&encoded, inner_base + 6),
        U256::from(11),
        "maxPriceGenPerTimeUnit"
    );
    assert_eq!(
        word(&encoded, inner_base + 7),
        U256::from(12),
        "storageFeeMaxGasPrice"
    );
    assert_eq!(
        word(&encoded, inner_base + 8),
        U256::from(13),
        "receiptFeeMaxGasPrice"
    );
    assert_eq!(
        word(&encoded, inner_base + 9),
        U256::from(3),
        "rotations length"
    );
}
