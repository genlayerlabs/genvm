//! EVM ABI encoding of the message-fee allocation tree into the chain's flat
//! parent-pointer `MessageAllocationNode[]` representation.

use genlayer_sdk::abi::gl_call::On;
use primitive_types::U256;

use super::{
    ExternalMessageParams, InternalMessageParams, MessageAllocationNode,
    MessageAllocationNodeParams,
};

/// Parent pointer used by top-level (root-layer) allocation nodes in the flat
/// chain representation: `type(uint256).max`. Any other value is a 0-based index
/// into the submitted array.
const NODE_ROOT_SENTINEL: U256 = U256::MAX;

/// Solidity `enum MessageType { External, Internal }`.
const MESSAGE_TYPE_EXTERNAL: u64 = 0;
const MESSAGE_TYPE_INTERNAL: u64 = 1;

/// Number of head words in the Solidity `MessageAllocationNode` tuple
/// (messageType, onAcceptance, parentIndex, recipient, callKey, budget, feeParams).
const NODE_HEAD_WORDS: usize = 7;

/// Number of head words in the Solidity `InternalMessageParams` tuple
/// (leader, validator, appealRounds, executionBudgetPerRound, rotations).
const INTERNAL_PARAMS_HEAD_WORDS: usize = 5;

fn push_u256(buf: &mut Vec<u8>, value: U256) {
    buf.extend_from_slice(&value.to_big_endian());
}

fn push_word(buf: &mut Vec<u8>, value: u64) {
    push_u256(buf, U256::from(value));
}

fn push_bool(buf: &mut Vec<u8>, value: bool) {
    push_word(buf, value as u64);
}

/// Left-pads a 20-byte address into a 32-byte ABI word.
fn push_address(buf: &mut Vec<u8>, addr: Option<&genlayer_sdk::calldata::Address>) {
    buf.extend_from_slice(&[0u8; 12]);
    match addr {
        Some(addr) => buf.extend_from_slice(&addr.raw()),
        None => buf.extend_from_slice(&[0u8; 20]),
    }
}

/// `callKey` is already a 32-byte word; `None` is the `CALL_KEY_WILDCARD` = `bytes32(0)`.
fn push_call_key(buf: &mut Vec<u8>, call_key: Option<&genlayer_sdk::abi::CallKey>) {
    match call_key {
        Some(ck) => buf.extend_from_slice(&ck.0),
        None => buf.extend_from_slice(&[0u8; 32]),
    }
}

/// ABI-encodes the nested allocation tree as the chain's flat
/// `MessageAllocationNode[]` representation (matching `abi.encode(nodes)`).
pub(super) fn encode(roots: &[MessageAllocationNode]) -> Vec<u8> {
    // Pre-order flatten: (node, parentIndex). Parents always precede children,
    // so the parent's array index is already assigned when a child is visited.
    let mut flat: Vec<(&MessageAllocationNode, U256)> = Vec::new();
    fn flatten<'a>(
        nodes: &'a [MessageAllocationNode],
        parent: U256,
        out: &mut Vec<(&'a MessageAllocationNode, U256)>,
    ) {
        for node in nodes {
            let my_index = U256::from(out.len() as u64);
            out.push((node, parent));
            flatten(&node.children, my_index, out);
        }
    }
    flatten(roots, NODE_ROOT_SENTINEL, &mut flat);

    // Encode each element as its own self-contained dynamic tuple.
    let elements: Vec<Vec<u8>> = flat
        .iter()
        .map(|(node, parent_index)| encode_node(node, *parent_index))
        .collect();

    // `abi.encode(MessageAllocationNode[])`: leading offset (0x20) to the array.
    let mut buf = Vec::new();
    push_word(&mut buf, 0x20);

    // Dynamic array of dynamic elements: length, then per-element head offsets
    // (relative to the start of the head region, i.e. right after the length),
    // then the element tails.
    push_word(&mut buf, elements.len() as u64);
    let mut offset = elements.len() * 32;
    for element in &elements {
        push_word(&mut buf, offset as u64);
        offset += element.len();
    }
    for element in &elements {
        buf.extend_from_slice(element);
    }

    buf
}

/// Encodes a single node as the Solidity `MessageAllocationNode` dynamic
/// tuple (offsets are relative to the start of this tuple).
fn encode_node(node: &MessageAllocationNode, parent_index: U256) -> Vec<u8> {
    let (message_type, on_acceptance, fee_params) = match &node.fee_params {
        MessageAllocationNodeParams::Internal(params) => (
            MESSAGE_TYPE_INTERNAL,
            matches!(node.on, On::Accepted),
            encode_internal_params(params),
        ),
        // External messages have no acceptance/finalize lifecycle.
        MessageAllocationNodeParams::External(params) => {
            (MESSAGE_TYPE_EXTERNAL, false, encode_external_params(params))
        }
    };

    let mut buf = Vec::new();
    push_word(&mut buf, message_type);
    push_bool(&mut buf, on_acceptance);
    push_u256(&mut buf, parent_index);
    push_address(&mut buf, node.recipient.as_ref());
    push_call_key(&mut buf, node.call_key.as_ref());
    push_u256(&mut buf, node.budget);
    // `feeParams` is the only dynamic field; its offset follows the head words.
    push_word(&mut buf, (NODE_HEAD_WORDS * 32) as u64);
    // Tail: `bytes feeParams` = length + 32-aligned data.
    push_word(&mut buf, fee_params.len() as u64);
    buf.extend_from_slice(&fee_params);
    let pad = fee_params.len().next_multiple_of(32) - fee_params.len();
    buf.extend(std::iter::repeat_n(0u8, pad));

    buf
}

/// `abi.encode(InternalMessageParams)` — a dynamic tuple (contains
/// `uint256[] rotations`), so it is prefixed with the offset word. `appealRounds`
/// is reconstructed as `len(rotations) - 1` (the chain-derived value).
fn encode_internal_params(params: &InternalMessageParams) -> Vec<u8> {
    let appeal_rounds = U256::from(params.rotations.len().saturating_sub(1) as u64);

    let mut buf = Vec::new();
    // `abi.encode` of a single dynamic struct prefixes the tuple with its offset.
    push_word(&mut buf, 0x20);
    push_u256(&mut buf, params.leader_timeunits_allocation);
    push_u256(&mut buf, params.validator_timeunits_allocation);
    push_u256(&mut buf, appeal_rounds);
    push_u256(&mut buf, params.execution_budget_per_round);
    // `rotations` is the only dynamic field; its offset follows the head words.
    push_word(&mut buf, (INTERNAL_PARAMS_HEAD_WORDS * 32) as u64);
    push_word(&mut buf, params.rotations.len() as u64);
    for rotation in &params.rotations {
        push_u256(&mut buf, *rotation);
    }

    buf
}

/// `abi.encode(ExternalMessageParams)` — a fully static tuple, so it is just
/// the two inline words with no leading offset.
fn encode_external_params(params: &ExternalMessageParams) -> Vec<u8> {
    let mut buf = Vec::new();
    push_u256(&mut buf, params.gas_limit);
    push_u256(&mut buf, params.max_gas_price);

    buf
}
