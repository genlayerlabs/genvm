//! Application Binary Interface for GenLayer contracts.
//!
//! This module provides the types, traits, and functions needed to build
//! GenLayer intelligent contracts in Rust.
//!
//! - [`consts`]: Auto-generated constants (EntryKind, ResultCode, etc.)
//! - [`entry`]: Contract entry point handling and the Contract trait
//! - [`gl_call`]: Message types for gl_call operations
//! - [`wasi`]: WASI bindings for storage, balance, and gl_call

use std::collections::BTreeMap;

use bytes::Bytes;

use crate::calldata;

pub mod consts;
pub mod entry;
pub mod gl_call;

#[cfg(feature = "wasi")]
pub mod wasi;

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields, tag = "type")]
pub enum ExecutionEmission {
    EthSend {
        address: calldata::Address,
        calldata: Bytes,
        value: primitive_types::U256,
    },
    PostMessage {
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
