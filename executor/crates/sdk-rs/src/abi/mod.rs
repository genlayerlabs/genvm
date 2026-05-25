//! Application Binary Interface for GenLayer contracts.
//!
//! This module provides the types, traits, and functions needed to build
//! GenLayer intelligent contracts in Rust.
//!
//! - [`consts`]: Auto-generated constants (EntryKind, ResultCode, etc.)
//! - [`entry`]: Contract entry point handling and the Contract trait
//! - [`gl_call`]: Message types for gl_call operations
//! - [`wasi`]: WASI bindings for storage, balance, and gl_call

use crate::calldata;
use bytes::Bytes;
use std::collections::BTreeMap;

#[cfg(feature = "arbitrary")]
pub(crate) mod arb;

pub mod consts;
pub mod entry;
pub mod gl_call;

#[cfg(feature = "wasi")]
pub mod wasi;

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    calldata::Encode,
    calldata::Decode,
    serde::Serialize,
    serde::Deserialize,
)]
pub struct CallKey(
    #[calldata(
        serialize_with = ::genlayer_calldata::codec::as_bytes::serialize,
        deserialize_with = ::genlayer_calldata::codec::as_bytes::deserialize,
    )]
    pub [u8; 32],
);

impl CallKey {
    pub const DEPLOY: CallKey = CallKey([0u8; 32]);
    pub const UNNAMED: CallKey = CallKey([0u8; 32]);

    pub fn for_method(name: &str) -> CallKey {
        use sha3::Digest;

        let name = name.as_bytes();
        let mut call_key = [0u8; 32];

        if name.len() < 32 {
            call_key[..name.len()].copy_from_slice(name);
        } else {
            let mut hasher = sha3::Keccak256::new();
            hasher.update(name);
            call_key.copy_from_slice(&hasher.finalize());
            call_key[31] |= 1;
        }

        CallKey(call_key)
    }

    pub fn as_u256(&self) -> primitive_types::U256 {
        primitive_types::U256::from_big_endian(&self.0)
    }

    pub fn from_u256(value: primitive_types::U256) -> CallKey {
        CallKey(value.to_big_endian())
    }
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, PartialEq, Eq, calldata::Encode, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields, tag = "type")]
#[calldata(tag = "type")]
pub enum ExecutionEmission {
    EthSend {
        address: calldata::Address,
        calldata: Bytes,
        value: primitive_types::U256,
    },
    PostMessage {
        call_key: CallKey,
        address: calldata::Address,
        calldata: calldata::Value,
        value: primitive_types::U256,
        on: gl_call::On,
    },
    DeployContract {
        calldata: calldata::Value,
        code: Bytes,
        value: primitive_types::U256,
        on: gl_call::On,
        salt_nonce: primitive_types::U256,
    },
    EmitEvent {
        topics: Vec<Bytes>,
        blob: BTreeMap<String, calldata::Value>,
    },
}
