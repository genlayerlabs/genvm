// This file is auto-generated. Do not edit!

#![allow(dead_code, clippy::redundant_static_lifetimes)]

use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
#[repr(u8)]
pub enum Methods {
    StorageRead = 0,
    StorageWrite = 1,
    ConsumeFuel = 2,
    EthCall = 3,
    GetBalance = 4,
    RemainingFuelAsGen = 5,
    NotifyNondetDisagreement = 6,
    GetLeaderNondetResult = 7,
    ConsumeResult = 8,
}

impl Methods {
    pub fn value(self) -> u8 {
        match self {
            Methods::StorageRead => 0,
            Methods::StorageWrite => 1,
            Methods::ConsumeFuel => 2,
            Methods::EthCall => 3,
            Methods::GetBalance => 4,
            Methods::RemainingFuelAsGen => 5,
            Methods::NotifyNondetDisagreement => 6,
            Methods::GetLeaderNondetResult => 7,
            Methods::ConsumeResult => 8,
        }
    }
    pub fn str_snake_case(self) -> &'static str {
        match self {
            Methods::StorageRead => "storage_read",
            Methods::StorageWrite => "storage_write",
            Methods::ConsumeFuel => "consume_fuel",
            Methods::EthCall => "eth_call",
            Methods::GetBalance => "get_balance",
            Methods::RemainingFuelAsGen => "remaining_fuel_as_gen",
            Methods::NotifyNondetDisagreement => "notify_nondet_disagreement",
            Methods::GetLeaderNondetResult => "get_leader_nondet_result",
            Methods::ConsumeResult => "consume_result",
        }
    }
}

impl TryFrom<u8> for Methods {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, ()> {
        match value {
            0 => Ok(Methods::StorageRead),
            1 => Ok(Methods::StorageWrite),
            2 => Ok(Methods::ConsumeFuel),
            3 => Ok(Methods::EthCall),
            4 => Ok(Methods::GetBalance),
            5 => Ok(Methods::RemainingFuelAsGen),
            6 => Ok(Methods::NotifyNondetDisagreement),
            7 => Ok(Methods::GetLeaderNondetResult),
            8 => Ok(Methods::ConsumeResult),
            _ => Err(()),
        }
    }
}
#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
#[repr(u8)]
pub enum Errors {
    Ok = 0,
    Absent = 1,
    Forbidden = 2,
    IAmLeader = 3,
    OutOfStorageGas = 4,
}

impl Errors {
    pub fn value(self) -> u8 {
        match self {
            Errors::Ok => 0,
            Errors::Absent => 1,
            Errors::Forbidden => 2,
            Errors::IAmLeader => 3,
            Errors::OutOfStorageGas => 4,
        }
    }
    pub fn str_snake_case(self) -> &'static str {
        match self {
            Errors::Ok => "ok",
            Errors::Absent => "absent",
            Errors::Forbidden => "forbidden",
            Errors::IAmLeader => "i_am_leader",
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
            3 => Ok(Errors::IAmLeader),
            4 => Ok(Errors::OutOfStorageGas),
            _ => Err(()),
        }
    }
}
