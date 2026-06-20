// This file is auto-generated. Do not edit!

#![allow(dead_code, clippy::redundant_static_lifetimes)]

use serde::{Deserialize, Serialize};

use std::borrow::Cow;

#[derive(
    Debug,
    PartialEq,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    ::genlayer_calldata::Encode,
    ::genlayer_calldata::Decode,
)]
#[repr(u8)]
pub enum ResultCode {
    Return = 0,
    UserError = 1,
    VmError = 2,
    InternalError = 3,
}

impl ResultCode {
    pub const SIZE: usize = 4;
    pub fn value(self) -> u8 {
        match self {
            ResultCode::Return => 0,
            ResultCode::UserError => 1,
            ResultCode::VmError => 2,
            ResultCode::InternalError => 3,
        }
    }
    pub fn str_snake_case(self) -> &'static str {
        match self {
            ResultCode::Return => "return",
            ResultCode::UserError => "user_error",
            ResultCode::VmError => "vm_error",
            ResultCode::InternalError => "internal_error",
        }
    }
}

impl TryFrom<u8> for ResultCode {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, ()> {
        match value {
            0 => Ok(ResultCode::Return),
            1 => Ok(ResultCode::UserError),
            2 => Ok(ResultCode::VmError),
            3 => Ok(ResultCode::InternalError),
            _ => Err(()),
        }
    }
}
#[derive(
    Debug,
    PartialEq,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    ::genlayer_calldata::Encode,
    ::genlayer_calldata::Decode,
)]
#[repr(u8)]
pub enum StorageType {
    Default = 0,
    LatestFinal = 1,
    LatestNonFinal = 2,
}

impl StorageType {
    pub const SIZE: usize = 3;
    pub fn value(self) -> u8 {
        match self {
            StorageType::Default => 0,
            StorageType::LatestFinal => 1,
            StorageType::LatestNonFinal => 2,
        }
    }
    pub fn str_snake_case(self) -> &'static str {
        match self {
            StorageType::Default => "default",
            StorageType::LatestFinal => "latest_final",
            StorageType::LatestNonFinal => "latest_non_final",
        }
    }
}

impl TryFrom<u8> for StorageType {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, ()> {
        match value {
            0 => Ok(StorageType::Default),
            1 => Ok(StorageType::LatestFinal),
            2 => Ok(StorageType::LatestNonFinal),
            _ => Err(()),
        }
    }
}
#[derive(
    Debug,
    PartialEq,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    ::genlayer_calldata::Encode,
    ::genlayer_calldata::Decode,
)]
#[repr(u8)]
pub enum EntryKind {
    Main = 0,
    Sandbox = 1,
    ConsensusStage = 2,
}

impl EntryKind {
    pub const SIZE: usize = 3;
    pub fn value(self) -> u8 {
        match self {
            EntryKind::Main => 0,
            EntryKind::Sandbox => 1,
            EntryKind::ConsensusStage => 2,
        }
    }
    pub fn str_snake_case(self) -> &'static str {
        match self {
            EntryKind::Main => "main",
            EntryKind::Sandbox => "sandbox",
            EntryKind::ConsensusStage => "consensus_stage",
        }
    }
}

impl TryFrom<u8> for EntryKind {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, ()> {
        match value {
            0 => Ok(EntryKind::Main),
            1 => Ok(EntryKind::Sandbox),
            2 => Ok(EntryKind::ConsensusStage),
            _ => Err(()),
        }
    }
}
pub mod memory_limiter_consts {
    pub const TABLE_ENTRY: u32 = 64;
    pub const FILE_MAPPING: u32 = 256;
    pub const FD_ALLOCATION: u32 = 96;
}

pub mod root_offsets {
    pub const CONTRACT: u32 = 0;
    pub const CODE: u32 = 1;
    pub const LOCKED_SLOTS: u32 = 2;
    pub const UPGRADERS: u32 = 3;
    pub const MAJOR: u32 = 4;
    pub const CODE_SLOT: u32 = 5;
}

pub mod top_limits {
    pub const NONDET_BLOCKS: u32 = 4096;
    pub const LOCKED_SLOTS: u32 = 256;
    pub const UPGRADERS: u32 = 32;
    pub const VM_RECURSION: u32 = 512;
    pub const WEB_REQUEST_MIN_SPACE: u32 = 65536;
    pub const WEB_RENDER_MIN_SPACE: u32 = 134217728;
    pub const MAX_FDS: u32 = 1024;
}

#[derive(
    Debug,
    PartialEq,
    Clone,
    Copy,
    Serialize,
    Deserialize,
    ::genlayer_calldata::Encode,
    ::genlayer_calldata::Decode,
)]
pub enum SpecialMethod {
    GetSchema,
    ErroredMessage,
}

impl SpecialMethod {
    pub fn value(self) -> &'static str {
        match self {
            SpecialMethod::GetSchema => "#get-schema",
            SpecialMethod::ErroredMessage => "#error",
        }
    }
    pub fn str_snake_case(self) -> &'static str {
        match self {
            SpecialMethod::GetSchema => "get_schema",
            SpecialMethod::ErroredMessage => "errored_message",
        }
    }
}

impl TryFrom<&str> for SpecialMethod {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, ()> {
        match value {
            "#get-schema" => Ok(SpecialMethod::GetSchema),
            "#error" => Ok(SpecialMethod::ErroredMessage),
            _ => Err(()),
        }
    }
}
#[allow(non_snake_case)]
#[rustfmt::skip]
pub mod __VmError {
    use std::borrow::Cow;
    use super::VmError;

    pub struct OomRam;

    impl OomRam {
        pub const fn val(&self) -> VmError { VmError(Cow::Borrowed("OOM RAM")) }
        pub const fn table(&self) -> VmError { VmError(Cow::Borrowed("OOM RAM table")) }
        pub const fn memory(&self) -> VmError { VmError(Cow::Borrowed("OOM RAM memory")) }
        pub const fn limit(&self) -> VmError { VmError(Cow::Borrowed("OOM RAM limit")) }
    }

    pub struct OomReceiptMessage;

    impl OomReceiptMessage {
        pub const fn internal(&self) -> VmError { VmError(Cow::Borrowed("OOM receipt message internal")) }
        pub const fn external(&self) -> VmError { VmError(Cow::Borrowed("OOM receipt message external")) }
    }

    pub struct OomReceipt;

    impl OomReceipt {
        pub const fn nondet_output(&self) -> VmError { VmError(Cow::Borrowed("OOM receipt nondet_output")) }
        pub const fn message(&self) -> OomReceiptMessage { OomReceiptMessage }
    }

    pub struct OomFees;

    impl OomFees {
        pub const fn internal(&self) -> VmError { VmError(Cow::Borrowed("OOM fees internal")) }
        pub const fn external(&self) -> VmError { VmError(Cow::Borrowed("OOM fees external")) }
    }

    pub struct Oom;

    impl Oom {
        pub const fn storage(&self) -> VmError { VmError(Cow::Borrowed("OOM storage")) }
        pub const fn ram(&self) -> OomRam { OomRam }
        pub const fn receipt(&self) -> OomReceipt { OomReceipt }
        pub const fn fees(&self) -> OomFees { OomFees }
    }

    pub struct InvalidContractWasm;

    impl InvalidContractWasm {
        pub const fn validating(&self) -> VmError { VmError(Cow::Borrowed("invalid_contract wasm validating")) }
        pub const fn linking(&self) -> VmError { VmError(Cow::Borrowed("invalid_contract wasm linking")) }
        pub const fn entrypoint(&self) -> VmError { VmError(Cow::Borrowed("invalid_contract wasm entrypoint")) }
    }

    pub struct InvalidContract;

    impl InvalidContract {
        pub const fn val(&self) -> VmError { VmError(Cow::Borrowed("invalid_contract")) }
        pub const fn absent_runner_comment(&self) -> VmError { VmError(Cow::Borrowed("invalid_contract absent_runner_comment")) }
        pub const fn not_utf8_text(&self) -> VmError { VmError(Cow::Borrowed("invalid_contract not_utf8_text")) }
        pub const fn malformed_runner(&self) -> VmError { VmError(Cow::Borrowed("invalid_contract malformed_runner")) }
        pub const fn major_mismatch(&self) -> VmError { VmError(Cow::Borrowed("invalid_contract major_mismatch")) }
        pub const fn wasm(&self) -> InvalidContractWasm { InvalidContractWasm }
    }

    pub struct ExitCode;

    impl ExitCode {
        pub fn val_i32(&self, v: i32) -> VmError {
            VmError(Cow::Owned(format!("exit_code {v}")))
        }
    }

    pub struct WasmTrap;

    impl WasmTrap {
        pub fn val_str(&self, v: &str) -> VmError {
            VmError(Cow::Owned(format!("wasm_trap {v}")))
        }
    }

    pub struct Host;

    impl Host {
        pub fn val_str(&self, v: &str) -> VmError {
            VmError(Cow::Owned(format!("host {v}")))
        }
    }

}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct VmError(pub Cow<'static, str>);

impl From<VmError> for String {
    fn from(val: VmError) -> String {
        val.0.into()
    }
}
#[rustfmt::skip]
impl VmError {
    pub const fn timeout() -> Self { Self(Cow::Borrowed("timeout")) }
    pub const fn absent_leader_nondet_output() -> Self { Self(Cow::Borrowed("absent_leader_nondet_output")) }
    pub const fn exit_code() -> __VmError::ExitCode { __VmError::ExitCode }
    pub const fn wasm_trap() -> __VmError::WasmTrap { __VmError::WasmTrap }
    pub const fn oom() -> __VmError::Oom { __VmError::Oom }
    pub const fn invalid_contract() -> __VmError::InvalidContract { __VmError::InvalidContract }
    pub const fn host() -> __VmError::Host { __VmError::Host }
}

pub const EVENT_MAX_TOPICS: u32 = 4;
