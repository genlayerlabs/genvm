use bytes::Bytes;
pub use genlayer_sdk::abi::entry::MessageData;
use primitive_types::U256;

/// Routing kind of a fee allocation node. No wildcard.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    genlayer_calldata::Encode,
    genlayer_calldata::Decode,
)]
pub enum MessageType {
    InternalAccepted,
    InternalFinalized,
    External,
}

/// TX-level / per-node fee parameters (mirrors the chain `MessageFeeParams`).
#[derive(
    Debug,
    Clone,
    serde::Serialize,
    serde::Deserialize,
    genlayer_calldata::Encode,
    genlayer_calldata::Decode,
)]
pub struct MessageFeeParams {
    pub leader_timeunits_allocation: U256,
    pub validator_timeunits_allocation: U256,
    /// chain: `rollupUnifiedBudgetPerRound`
    pub execution_budget_per_round: U256,
    pub rotations: Vec<U256>,
}

/// One node of the message-fee allocation tree.
#[derive(
    Debug,
    Clone,
    serde::Serialize,
    serde::Deserialize,
    genlayer_calldata::Encode,
    genlayer_calldata::Decode,
)]
pub struct MessageFeeAllocationNode {
    /// External | Internal — distinguishes routing; no wildcard.
    pub message_type: MessageType,
    /// `None` for a root-layer node; otherwise the parent's index in the
    /// allocation array (chain sentinel: `NODE_ROOT_SENTINEL`).
    pub parent_index: Option<u64>,
    /// Target contract.
    pub recipient: Option<genlayer_sdk::calldata::Address>,
    /// `None` = wildcard: all call keys for this recipient
    /// (chain sentinel: `CALL_KEY_WILDCARD` = `bytes32(0)`).
    pub call_key: Option<genlayer_sdk::abi::CallKey>,
    /// Max budget for matching messages.
    pub budget: U256,
    /// Same structure as TX-level params.
    pub fee_params: MessageFeeParams,
}

#[allow(clippy::enum_variant_names)]
#[derive(
    Debug, Clone, PartialEq, Eq, genlayer_calldata::Encode, serde::Serialize, serde::Deserialize,
)]
#[serde(deny_unknown_fields, tag = "type")]
#[calldata(tag = "type")]
pub enum ExecutionEmission {
    EthSend {
        address: genlayer_sdk::calldata::Address,
        calldata: Bytes,
        value: U256,
        message_fee: U256,
        receipt_fee: U256,
    },
    PostMessage {
        call_key: genlayer_sdk::abi::CallKey,
        address: genlayer_sdk::calldata::Address,
        calldata: genlayer_sdk::calldata::Value,
        value: U256,
        on: genlayer_sdk::abi::gl_call::On,
        message_fee: U256,
        receipt_fee: U256,
    },
    DeployContract {
        calldata: genlayer_sdk::calldata::Value,
        code: Bytes,
        value: U256,
        on: genlayer_sdk::abi::gl_call::On,
        salt_nonce: U256,
        message_fee: U256,
        receipt_fee: U256,
    },
    EmitEvent {
        topics: Vec<Bytes>,
        blob: std::collections::BTreeMap<String, genlayer_sdk::calldata::Value>,
        storage_fee: U256,
    },
}

impl MessageFeeAllocationNode {
    pub fn matches(
        &self,
        message_type: MessageType,
        recipient: genlayer_sdk::calldata::Address,
        call_key: genlayer_sdk::abi::CallKey,
    ) -> bool {
        if self.message_type != message_type {
            false
        } else if self.recipient.as_ref().is_some_and(|r| *r != recipient) {
            false
        } else {
            !self.call_key.as_ref().is_some_and(|ck| *ck != call_key)
        }
    }
}

#[derive(
    Debug,
    Clone,
    serde::Serialize,
    serde::Deserialize,
    genlayer_calldata::Encode,
    genlayer_calldata::Decode,
)]
pub struct ExecutionData {
    pub calldata: Bytes,
    pub message: MessageData,
    pub host_data: String,
    pub code: Option<Bytes>,
    pub leader_nondet_results: Option<Vec<Bytes>>,
    /// Maps each host method (by index) to a host id. When empty, all methods use host 0.
    pub method_hosts: Vec<u8>,
    pub bucket_totals: Vec<num_bigint::BigInt>,
    /// Host-provided `node` fee constants (moved off `host_data`).
    pub gas_data: std::collections::BTreeMap<String, String>,
    /// Message-fee allocation tree passed alongside the execution.
    pub message_fee_allocation: Vec<MessageFeeAllocationNode>,
    /// Initial time-unit budget for this execution.
    pub initial_time_units_allocation: u32,
}
