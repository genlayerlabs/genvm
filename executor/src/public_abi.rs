#![allow(dead_code, clippy::redundant_static_lifetimes)]

use serde_derive::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
#[repr(u8)]
pub enum ResultCode {
    Return = 0,
    UserError = 1,
    VmError = 2,
    InternalError = 3,
}

impl ResultCode {
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
#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
#[repr(u8)]
pub enum StorageType {
    Default = 0,
    LatestFinal = 1,
    LatestNonFinal = 2,
}

impl StorageType {
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
#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
#[repr(u8)]
pub enum EntryKind {
    Main = 0,
    Sandbox = 1,
    ConsensusStage = 2,
}

impl EntryKind {
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
#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
#[repr(u32)]
pub enum MemoryLimiterConsts {
    TableEntry = 64,
    FileMapping = 256,
}

impl MemoryLimiterConsts {
    pub fn value(self) -> u32 {
        match self {
            MemoryLimiterConsts::TableEntry => 64,
            MemoryLimiterConsts::FileMapping => 256,
        }
    }
    pub fn str_snake_case(self) -> &'static str {
        match self {
            MemoryLimiterConsts::TableEntry => "table_entry",
            MemoryLimiterConsts::FileMapping => "file_mapping",
        }
    }
}

impl TryFrom<u32> for MemoryLimiterConsts {
    type Error = ();

    fn try_from(value: u32) -> Result<Self, ()> {
        match value {
            64 => Ok(MemoryLimiterConsts::TableEntry),
            256 => Ok(MemoryLimiterConsts::FileMapping),
            _ => Err(()),
        }
    }
}
#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
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
#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub enum VmError {
    Timeout,
    ValidatorDisagrees,
    VersionTooBig,
    Oom,
    InvalidContract,
}

impl VmError {
    pub fn value(self) -> &'static str {
        match self {
            VmError::Timeout => "timeout",
            VmError::ValidatorDisagrees => "validator_disagrees",
            VmError::VersionTooBig => "version_too_big",
            VmError::Oom => "OOM",
            VmError::InvalidContract => "invalid_contract",
        }
    }
    pub fn str_snake_case(self) -> &'static str {
        match self {
            VmError::Timeout => "timeout",
            VmError::ValidatorDisagrees => "validator_disagrees",
            VmError::VersionTooBig => "version_too_big",
            VmError::Oom => "oom",
            VmError::InvalidContract => "invalid_contract",
        }
    }
}

impl TryFrom<&str> for VmError {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, ()> {
        match value {
            "timeout" => Ok(VmError::Timeout),
            "validator_disagrees" => Ok(VmError::ValidatorDisagrees),
            "version_too_big" => Ok(VmError::VersionTooBig),
            "OOM" => Ok(VmError::Oom),
            "invalid_contract" => Ok(VmError::InvalidContract),
            _ => Err(()),
        }
    }
}
pub const EVENT_MAX_TOPICS: u32 = 4;
pub const ABSENT_VERSION: &'static str = "v0.1.0";
pub const CODE_SLOT_OFFSET: u32 = 1;
