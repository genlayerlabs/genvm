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

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, calldata::Encode)]
#[serde(deny_unknown_fields, tag = "type")]
#[calldata(tag = "type")]
pub enum ExecutionEmission {
    EthSend {
        address: calldata::Address,
        calldata: Bytes,
        value: primitive_types::U256,
    },
    PostMessage {
        call_key: primitive_types::U256,
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

pub mod call_key {
    pub const DEPLOY: primitive_types::U256 = primitive_types::U256::zero();
    pub const UNNAMED: primitive_types::U256 = primitive_types::U256::zero();

    pub fn for_method(name: &str) -> primitive_types::U256 {
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

        primitive_types::U256::from_big_endian(&call_key)
    }
}
