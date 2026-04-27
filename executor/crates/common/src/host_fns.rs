// This file is auto-generated. Do not edit!

#![allow(dead_code, clippy::redundant_static_lifetimes)]

use serde::{Deserialize, Serialize};

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
pub enum Methods {
    StorageRead = 0,
    ConsumeFuel = 1,
    EthCall = 2,
    GetBalance = 3,
    RemainingFuelAsGen = 4,
    NotifyNondetDisagreement = 5,
    ConsumeResult = 6,
    NotifyFinished = 7,
}

impl Methods {
    pub const SIZE: usize = 8;
    pub fn value(self) -> u8 {
        match self {
            Methods::StorageRead => 0,
            Methods::ConsumeFuel => 1,
            Methods::EthCall => 2,
            Methods::GetBalance => 3,
            Methods::RemainingFuelAsGen => 4,
            Methods::NotifyNondetDisagreement => 5,
            Methods::ConsumeResult => 6,
            Methods::NotifyFinished => 7,
        }
    }
    pub fn str_snake_case(self) -> &'static str {
        match self {
            Methods::StorageRead => "storage_read",
            Methods::ConsumeFuel => "consume_fuel",
            Methods::EthCall => "eth_call",
            Methods::GetBalance => "get_balance",
            Methods::RemainingFuelAsGen => "remaining_fuel_as_gen",
            Methods::NotifyNondetDisagreement => "notify_nondet_disagreement",
            Methods::ConsumeResult => "consume_result",
            Methods::NotifyFinished => "notify_finished",
        }
    }
}

impl TryFrom<u8> for Methods {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, ()> {
        match value {
            0 => Ok(Methods::StorageRead),
            1 => Ok(Methods::ConsumeFuel),
            2 => Ok(Methods::EthCall),
            3 => Ok(Methods::GetBalance),
            4 => Ok(Methods::RemainingFuelAsGen),
            5 => Ok(Methods::NotifyNondetDisagreement),
            6 => Ok(Methods::ConsumeResult),
            7 => Ok(Methods::NotifyFinished),
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
pub enum Errors {
    Ok = 0,
    Absent = 1,
    Forbidden = 2,
    OutOfStorageGas = 3,
}

impl Errors {
    pub const SIZE: usize = 4;
    pub fn value(self) -> u8 {
        match self {
            Errors::Ok => 0,
            Errors::Absent => 1,
            Errors::Forbidden => 2,
            Errors::OutOfStorageGas => 3,
        }
    }
    pub fn str_snake_case(self) -> &'static str {
        match self {
            Errors::Ok => "ok",
            Errors::Absent => "absent",
            Errors::Forbidden => "forbidden",
            Errors::OutOfStorageGas => "out_of_storage_gas",
        }
    }
}

impl TryFrom<u8> for Errors {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, ()> {
        match value {
            0 => Ok(Errors::Ok),
            1 => Ok(Errors::Absent),
            2 => Ok(Errors::Forbidden),
            3 => Ok(Errors::OutOfStorageGas),
            _ => Err(()),
        }
    }
}
pub const CURRENT_MAJOR: u8 = 0;
pub const CURRENT_MAJOR_STR: &'static str = "v0.0.0";
