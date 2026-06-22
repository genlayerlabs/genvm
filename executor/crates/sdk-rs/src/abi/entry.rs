//! Contract entry point handling and message types.
//!
//! This module provides the types and traits needed to implement GenLayer contracts,
//! including message parsing, entry point handling, and the contract trait definitions.

use std::collections::BTreeMap;

use crate::abi::consts::{EntryKind, ResultCode};
use crate::calldata::{self, Address, Map, Value};
use serde::{Deserialize, Serialize};

fn decode_datetime_rfc3339(
    val: calldata::Value,
) -> Result<chrono::DateTime<chrono::Utc>, calldata::codec::DecodeError> {
    let calldata::Value::Str(s) = val else {
        return Err(calldata::codec::DecodeError::Unexpected(
            "expected string for datetime",
        ));
    };
    chrono::DateTime::parse_from_rfc3339(&s)
        .map(|dt| dt.to_utc())
        .map_err(|e| calldata::codec::DecodeError::UserError(Box::new(e)))
}

fn encode_entry_kind<W: calldata::Writer>(
    ek: &EntryKind,
    enc: &mut calldata::Encoder<W>,
) -> Result<(), W::Error> {
    enc.push_u64(*ek as u64)
}

fn decode_entry_kind(val: calldata::Value) -> Result<EntryKind, calldata::codec::DecodeError> {
    let calldata::Value::Number(n) = val else {
        return Err(calldata::codec::DecodeError::Unexpected(
            "expected number for EntryKind",
        ));
    };
    let v: u8 = <u8 as TryFrom<&num_bigint::BigInt>>::try_from(&n).map_err(|_| {
        calldata::codec::DecodeError::OutOfRange {
            value: n.to_string(),
            target: "EntryKind",
        }
    })?;
    EntryKind::try_from(v).map_err(|_| calldata::codec::DecodeError::OutOfRange {
        value: v.to_string(),
        target: "EntryKind",
    })
}

fn encode_datetime_rfc3339<W: calldata::Writer>(
    dt: &chrono::DateTime<chrono::Utc>,
    enc: &mut calldata::Encoder<W>,
) -> Result<(), W::Error> {
    use chrono::SecondsFormat;
    let s = dt.to_rfc3339_opts(SecondsFormat::AutoSi, true);
    enc.push_str(&s)
}

fn entry_kind_as_int<S>(data: &EntryKind, s: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    s.serialize_u8(*data as u8)
}

fn entry_kind_from_int<'de, D>(deserializer: D) -> Result<EntryKind, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct Visitor;

    impl serde::de::Visitor<'_> for Visitor {
        type Value = EntryKind;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("an integer 0, 1, or 2")
        }

        fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            match v {
                0 => Ok(EntryKind::Main),
                1 => Ok(EntryKind::Sandbox),
                2 => Ok(EntryKind::ConsensusStage),
                _ => Err(E::invalid_value(
                    serde::de::Unexpected::Unsigned(v),
                    &"0, 1, or 2",
                )),
            }
        }

        fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            self.visit_u64(v as u64)
        }
    }

    deserializer.deserialize_any(Visitor)
}

fn default_stack() -> Vec<Address> {
    Vec::new()
}

fn default_bigint() -> num_bigint::BigInt {
    num_bigint::BigInt::default()
}

fn default_datetime() -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::parse_from_rfc3339("2024-11-26T06:42:42.424242Z")
        .unwrap()
        .to_utc()
}

/// Core message data that represents the transaction context.
/// This is the minimal set of information needed to process a contract call.
#[derive(Debug, Clone, Serialize, Deserialize, calldata::Encode, calldata::Decode)]
pub struct MessageData {
    pub contract_address: Address,
    pub sender_address: Address,
    pub origin_address: Address,
    /// View methods call chain.
    /// It is empty for entrypoint (refer to [`contract_address`])
    #[serde(default)]
    #[calldata(default = default_stack)]
    pub stack: Vec<Address>,

    pub chain_id: num_bigint::BigInt,
    #[serde(default)]
    #[calldata(default = default_bigint)]
    pub value: num_bigint::BigInt,
    pub is_init: bool,
    /// Transaction timestamp
    #[serde(default = "default_datetime")]
    #[calldata(
        serialize_with = encode_datetime_rfc3339,
        deserialize_with = decode_datetime_rfc3339
    )]
    #[calldata(default = default_datetime)]
    pub datetime: chrono::DateTime<chrono::Utc>,
}

#[cfg(feature = "arbitrary")]
impl<'a> arbitrary::Arbitrary<'a> for MessageData {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        use arbitrary::Arbitrary;

        let ts = u32::arbitrary(u)?;
        let Some(datetime) = chrono::DateTime::<chrono::Utc>::from_timestamp_secs(ts as i64) else {
            return Err(arbitrary::Error::NotEnoughData);
        };

        let chain_id_bytes: [u8; 32] = Arbitrary::arbitrary(u)?;
        let chain_id = primitive_types::U256::from_big_endian(&chain_id_bytes);

        let value_bytes: [u8; 32] = Arbitrary::arbitrary(u)?;
        let value = primitive_types::U256::from_big_endian(&value_bytes);

        Ok(Self {
            contract_address: Arbitrary::arbitrary(u)?,
            sender_address: Arbitrary::arbitrary(u)?,
            origin_address: Arbitrary::arbitrary(u)?,
            stack: Vec::new(),
            chain_id: num_bigint::BigInt::from_bytes_be(
                num_bigint::Sign::Plus,
                &chain_id.to_big_endian(),
            ),
            value: num_bigint::BigInt::from_bytes_be(
                num_bigint::Sign::Plus,
                &value.to_big_endian(),
            ),
            is_init: bool::arbitrary(u)?,
            datetime,
        })
    }
}

/// Flat struct with all fields manually listed (no flatten) to avoid serde's Content buffering
/// which breaks type-name-based Address handling during deserialization.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, calldata::Encode, calldata::Decode)]
pub struct ExtendedMessageFlat {
    // MessageData fields
    pub contract_address: Address,
    pub sender_address: Address,
    pub origin_address: Address,
    #[serde(default)]
    #[calldata(default = default_stack)]
    pub stack: Vec<Address>,
    pub chain_id: num_bigint::BigInt,
    #[serde(default)]
    #[calldata(default = default_bigint)]
    pub value: num_bigint::BigInt,
    pub is_init: bool,
    #[serde(default = "default_datetime")]
    #[calldata(
        serialize_with = encode_datetime_rfc3339,
        deserialize_with = decode_datetime_rfc3339
    )]
    #[calldata(default = default_datetime)]
    pub datetime: chrono::DateTime<chrono::Utc>,

    // ExtendedMessage-specific fields
    #[serde(
        serialize_with = "entry_kind_as_int",
        deserialize_with = "entry_kind_from_int"
    )]
    #[calldata(
        serialize_with = encode_entry_kind,
        deserialize_with = decode_entry_kind
    )]
    pub entry_kind: EntryKind,
    pub entry_data: bytes::Bytes,
    pub entry_stage_data: Value,
}

impl From<ExtendedMessageFlat> for ExtendedMessage {
    fn from(flat: ExtendedMessageFlat) -> Self {
        ExtendedMessage {
            message: MessageData {
                contract_address: flat.contract_address,
                sender_address: flat.sender_address,
                origin_address: flat.origin_address,
                stack: flat.stack,
                chain_id: flat.chain_id,
                value: flat.value,
                is_init: flat.is_init,
                datetime: flat.datetime,
            },
            entry_kind: flat.entry_kind,
            entry_data: flat.entry_data,
            entry_stage_data: flat.entry_stage_data,
        }
    }
}

impl From<ExtendedMessage> for ExtendedMessageFlat {
    fn from(msg: ExtendedMessage) -> Self {
        ExtendedMessageFlat {
            contract_address: msg.message.contract_address,
            sender_address: msg.message.sender_address,
            origin_address: msg.message.origin_address,
            stack: msg.message.stack,
            chain_id: msg.message.chain_id,
            value: msg.message.value,
            is_init: msg.message.is_init,
            datetime: msg.message.datetime,
            entry_kind: msg.entry_kind,
            entry_data: msg.entry_data,
            entry_stage_data: msg.entry_stage_data,
        }
    }
}

/// Extended message that includes entry point information.
/// This is the full message passed to WebAssembly contracts via stdin.
#[derive(Debug, Clone, Serialize, calldata::Encode)]
pub struct ExtendedMessage {
    pub message: MessageData,

    #[serde(serialize_with = "entry_kind_as_int")]
    #[calldata(serialize_with = encode_entry_kind)]
    pub entry_kind: EntryKind,
    pub entry_data: bytes::Bytes,

    pub entry_stage_data: Value,
}

impl<'de> Deserialize<'de> for ExtendedMessage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        ExtendedMessageFlat::deserialize(deserializer).map(Into::into)
    }
}

/// Contract definition types and traits.
///
/// This module contains the core traits and types for implementing GenLayer contracts.
pub mod contract_def {
    use super::*;

    type UserError = String;

    /// Parsed calldata for Main entry kind - regular contract method calls.
    #[derive(Debug, Clone, Default)]
    pub struct MainCalldata {
        /// Method name to call (empty string for default/receive)
        pub method: String,
        /// Positional arguments
        pub args: Vec<Value>,
        /// Keyword arguments
        pub kwargs: Map,
    }

    impl MainCalldata {
        /// Parse calldata bytes into MainCalldata
        pub fn parse(data: &[u8]) -> Result<Self, calldata::BinDecodeError> {
            let value = calldata::decode(data)?;

            let Value::Map(map) = value else {
                return Ok(Self::default());
            };

            let method = map
                .get("method")
                .and_then(|v| v.as_str())
                .map(|s| s.to_owned())
                .unwrap_or_default();

            let args = match map.get("args") {
                Some(Value::Array(arr)) => arr.clone(),
                _ => Vec::new(),
            };

            let kwargs = match map.get("kwargs") {
                Some(Value::Map(m)) => m.clone(),
                _ => BTreeMap::new(),
            };

            Ok(Self {
                method,
                args,
                kwargs,
            })
        }
    }

    /// Core trait for implementing GenLayer contracts.
    ///
    /// Implement this trait to define how your contract handles different entry points.
    pub trait Contract {
        /// Handle a Main entry - regular contract method calls.
        fn handle_main(
            &mut self,
            message: MessageData,
            data: bytes::Bytes,
        ) -> Result<calldata::Value, UserError>;

        /// Handle a Sandbox entry - contract decides how to handle the payload.
        fn handle_sandbox(
            &mut self,
            message: MessageData,
            data: bytes::Bytes,
        ) -> Result<Vec<u8>, UserError>;

        /// Handle a ConsensusStage entry - validator consensus functions.
        /// - For leader nodes: stage_data is null
        /// - For validator nodes: stage_data contains {leaders_result: <calldata>}
        fn handle_consensus_stage(
            &mut self,
            message: MessageData,
            data: bytes::Bytes,
            stage_data: ConsensusStageData,
        ) -> Result<calldata::Value, UserError>;
    }

    /// Error type for parsing consensus stage data
    #[derive(Debug)]
    pub enum ConsensusStageParseError {
        /// Invalid format for stage data (expected null or map with bytes)
        InvalidFormat,
        /// Missing leaders_result field in validator data
        MissingLeadersResult,
        /// Empty leaders_result data
        EmptyLeadersResult,
        /// Unknown result code
        UnknownResultCode(u8),
        /// Failed to decode calldata
        DecodeError(String),
        /// Invalid UTF-8 in error message
        InvalidUtf8(String),
    }

    impl std::fmt::Display for ConsensusStageParseError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                ConsensusStageParseError::InvalidFormat => {
                    write!(
                        f,
                        "invalid stage data format (expected null or map with bytes)"
                    )
                }
                ConsensusStageParseError::MissingLeadersResult => {
                    write!(f, "missing leaders_result field in validator data")
                }
                ConsensusStageParseError::EmptyLeadersResult => {
                    write!(f, "empty leaders_result")
                }
                ConsensusStageParseError::UnknownResultCode(code) => {
                    write!(f, "unknown result code: {}", code)
                }
                ConsensusStageParseError::DecodeError(e) => write!(f, "decode error: {}", e),
                ConsensusStageParseError::InvalidUtf8(e) => write!(f, "invalid UTF-8: {}", e),
            }
        }
    }

    impl std::error::Error for ConsensusStageParseError {}

    /// Parsed result from leader
    #[derive(Debug, Clone)]
    pub enum LeaderResult {
        Return(Value),
        UserError(Value),
        VmError(String),
    }

    impl LeaderResult {
        /// Parse leader result from raw bytes (result code prefix + payload)
        pub fn parse(data: &[u8]) -> Result<Self, ConsensusStageParseError> {
            if data.is_empty() {
                return Err(ConsensusStageParseError::EmptyLeadersResult);
            }

            let code = ResultCode::try_from(data[0])
                .map_err(|()| ConsensusStageParseError::UnknownResultCode(data[0]))?;
            let rest = &data[1..];

            match code {
                ResultCode::Return => {
                    let value = calldata::decode(rest)
                        .map_err(|e| ConsensusStageParseError::DecodeError(e.to_string()))?;
                    Ok(LeaderResult::Return(value))
                }
                ResultCode::UserError => {
                    if rest.len() < 4 || rest[..4] != [0u8; 4] {
                        return Err(ConsensusStageParseError::DecodeError(
                            "missing tagged calldata prefix in user error".to_owned(),
                        ));
                    }
                    let value = calldata::decode(&rest[4..])
                        .map_err(|e| ConsensusStageParseError::DecodeError(e.to_string()))?;
                    Ok(LeaderResult::UserError(value))
                }
                ResultCode::VmError | ResultCode::InternalError => {
                    let msg = String::from_utf8(rest.to_vec())
                        .map_err(|e| ConsensusStageParseError::InvalidUtf8(e.to_string()))?;
                    Ok(LeaderResult::VmError(msg))
                }
            }
        }
    }

    /// Consensus stage data passed to contracts during consensus operations.
    #[derive(Debug, Clone)]
    pub enum ConsensusStageData {
        /// This node is the leader - no previous result available.
        Leader,
        /// This node is a validator - leader's result is provided for verification.
        Validator { leaders_result: LeaderResult },
    }

    impl ConsensusStageData {
        /// Parse consensus stage data from a calldata Value.
        pub fn parse(value: Value) -> Result<Self, ConsensusStageParseError> {
            match value {
                Value::Null => Ok(ConsensusStageData::Leader),
                Value::Map(mut map) => {
                    let leaders_result_value = map
                        .remove("leaders_result")
                        .ok_or(ConsensusStageParseError::MissingLeadersResult)?;

                    let data = match leaders_result_value {
                        Value::Bytes(bytes) => bytes,
                        _ => return Err(ConsensusStageParseError::InvalidFormat),
                    };

                    let leaders_result = LeaderResult::parse(&data)?;

                    Ok(ConsensusStageData::Validator { leaders_result })
                }
                _ => Err(ConsensusStageParseError::InvalidFormat),
            }
        }
    }

    /// Extended contract trait with separate deploy and method handlers.
    ///
    /// This trait provides a more ergonomic API by splitting `handle_main` into
    /// `handle_deploy` (for initialization) and `handle_method` (for regular calls).
    pub trait ContractExt: Contract {
        type Error: std::string::ToString;

        /// Handle contract deployment/initialization.
        fn handle_deploy(
            &mut self,
            message: MessageData,
            args: Vec<calldata::Value>,
            kwargs: calldata::Map,
        ) -> Result<calldata::Value, Self::Error>;

        /// Handle a method call on the contract.
        fn handle_method(
            &mut self,
            message: MessageData,
            method: String,
            args: Vec<calldata::Value>,
            kwargs: calldata::Map,
        ) -> Result<calldata::Value, Self::Error>;

        /// Default implementation that delegates to handle_deploy or handle_method.
        fn handle_main(
            &mut self,
            message: MessageData,
            data: Vec<u8>,
        ) -> Result<calldata::Value, UserError> {
            let calldata =
                MainCalldata::parse(&data).map_err(|e| calldata::BinDecodeError::to_string(&e))?;

            if message.is_init {
                self.handle_deploy(message, calldata.args, calldata.kwargs)
            } else {
                self.handle_method(message, calldata.method, calldata.args, calldata.kwargs)
            }
            .map_err(|e| e.to_string())
        }
    }

    /// Result type for contract execution.
    pub enum ContractResult<T> {
        /// Successful return with value
        Return(T),
        /// User error with message
        UserError(String),
    }

    /// Error type for contract_main
    #[derive(Debug)]
    pub enum ContractMainError {
        /// Failed to read stdin
        StdinRead(std::io::Error),
        /// Failed to decode extended message
        MessageDecode(calldata::codec::DecodeError),
        /// Invalid entry kind value
        InvalidEntryKind(u8),
        /// Failed to parse stage data
        InvalidStageDataFormat(ConsensusStageParseError),
        Custom(String),
    }

    impl From<calldata::codec::DecodeError> for ContractMainError {
        fn from(e: calldata::codec::DecodeError) -> Self {
            ContractMainError::MessageDecode(e)
        }
    }

    impl From<std::io::Error> for ContractMainError {
        fn from(e: std::io::Error) -> Self {
            ContractMainError::StdinRead(e)
        }
    }

    impl std::error::Error for ContractMainError {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                ContractMainError::StdinRead(e) => Some(e),
                ContractMainError::MessageDecode(e) => Some(e),
                ContractMainError::InvalidEntryKind(_) => None,
                ContractMainError::InvalidStageDataFormat(e) => Some(e),
                ContractMainError::Custom(_) => None,
            }
        }
    }

    impl std::fmt::Display for ContractMainError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                ContractMainError::StdinRead(e) => write!(f, "failed to read stdin: {}", e),
                ContractMainError::MessageDecode(e) => {
                    write!(f, "failed to decode message: {:?}", e)
                }
                ContractMainError::InvalidEntryKind(k) => write!(f, "invalid entry kind: {}", k),
                ContractMainError::InvalidStageDataFormat(e) => {
                    write!(f, "invalid stage data format: {}", e)
                }
                ContractMainError::Custom(msg) => f.write_str(msg),
            }
        }
    }
}

/// WASI-specific functions for contract execution.
///
/// These functions are only available when the `wasi` feature is enabled.
#[cfg(feature = "wasi")]
pub mod wasi_only {
    use super::*;

    /// Immediately return a value and terminate the contract.
    ///
    /// This function does not return - it sends the Return message via gl_call
    /// and the process terminates.
    pub fn return_immediately(value: calldata::Value) -> ! {
        let last_gl_call_data = calldata::Value::Map(std::collections::BTreeMap::from([(
            "Return".to_owned(),
            value,
        )]));

        let last_gl_call_data = calldata::encode(&last_gl_call_data);
        let _ = crate::abi::wasi::gl_call(&last_gl_call_data);
        std::unreachable!();
    }

    /// Immediately trigger a user error and terminate the contract.
    ///
    /// This function does not return - it sends a UserError message via gl_call
    /// and the process terminates.
    pub fn user_error_immediately(msg: String) -> ! {
        let last_gl_call_data = calldata::Value::Map(std::collections::BTreeMap::from([(
            "UserError".to_owned(),
            crate::calldata::Value::Str(msg),
        )]));

        let last_gl_call_data = calldata::encode(&last_gl_call_data);
        let _ = crate::abi::wasi::gl_call(&last_gl_call_data);
        std::unreachable!();
    }

    /// Internal implementation of contract_main that returns a Result.
    ///
    /// Reads the extended message from stdin, dispatches to the appropriate
    /// handler based on entry_kind, and returns the result.
    pub fn contract_main_impl<H>() -> Result<calldata::Value, contract_def::ContractMainError>
    where
        H: contract_def::Contract + Default,
    {
        use contract_def::*;
        use std::io::Read;

        // Read all of stdin
        let mut input = Vec::new();
        std::io::stdin()
            .read_to_end(&mut input)
            .map_err(ContractMainError::StdinRead)?;

        let value = calldata::decode(&input).map_err(|e| {
            ContractMainError::MessageDecode(calldata::codec::DecodeError::BinDecodeError(e))
        })?;
        std::mem::drop(input);

        let extended: ExtendedMessageFlat =
            calldata::from_value(value).map_err(ContractMainError::MessageDecode)?;

        let extended: ExtendedMessage = extended.into();

        let mut handler = H::default();

        let result = match extended.entry_kind {
            EntryKind::Sandbox => {
                let sb_result = handler
                    .handle_sandbox(extended.message, extended.entry_data)
                    .map_err(ContractMainError::Custom)?;

                return Ok(calldata::Value::Bytes(sb_result));
            }
            EntryKind::Main => handler
                .handle_main(extended.message, extended.entry_data)
                .map_err(ContractMainError::Custom)?,
            EntryKind::ConsensusStage => {
                let stage_data = ConsensusStageData::parse(extended.entry_stage_data)
                    .map_err(ContractMainError::InvalidStageDataFormat)?;

                handler
                    .handle_consensus_stage(extended.message, extended.entry_data, stage_data)
                    .map_err(ContractMainError::Custom)?
            }
        };

        Ok(result)
    }

    /// Main entry point for contract execution.
    ///
    /// Calls `contract_main_impl` and sends the result (or error) via gl_call.
    /// This function does not return.
    pub fn contract_main<H>()
    where
        H: contract_def::Contract + Default,
    {
        let last_gl_call_data = match contract_main_impl::<H>() {
            Ok(value) => calldata::Value::Map(std::collections::BTreeMap::from([(
                "Return".to_owned(),
                value,
            )])),
            Err(e) => calldata::Value::Map(std::collections::BTreeMap::from([(
                "UserError".to_owned(),
                crate::calldata::Value::Str(e.to_string()),
            )])),
        };

        let last_gl_call_data = calldata::encode(&last_gl_call_data);
        let _ = crate::abi::wasi::gl_call(&last_gl_call_data);
        std::unreachable!();
    }

    /// Macro to generate the main function for a contract.
    ///
    /// # Example
    /// ```ignore
    /// use genlayer_sdk::contract_main;
    ///
    /// struct MyContract;
    /// impl Contract for MyContract { /* ... */ }
    ///
    /// contract_main!(MyContract);
    /// ```
    #[macro_export]
    macro_rules! contract_main {
        ($handler:ty) => {
            fn main() {
                $crate::abi::entry::wasi_only::contract_main::<$handler>();
            }
        };
    }
}

#[cfg(feature = "wasi")]
pub use wasi_only::*;
