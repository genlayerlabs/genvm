use serde_derive::{Deserialize, Serialize};
#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
#[repr(u8)]
pub enum Methods {
    GetCalldata = 0,
    StorageRead = 1,
    StorageWrite = 2,
    ConsumeResult = 3,
    GetLeaderNondetResult = 4,
    PostNondetResult = 5,
    PostMessage = 6,
    PostEvent = 7,
    ConsumeFuel = 8,
    DeployContract = 9,
    EthCall = 10,
    EthSend = 11,
    GetBalance = 12,
}

#[allow(dead_code)]
impl Methods {
    pub fn str_snake_case(self) -> &'static str {
        match self {
            Methods::GetCalldata => "get_calldata",
            Methods::StorageRead => "storage_read",
            Methods::StorageWrite => "storage_write",
            Methods::ConsumeResult => "consume_result",
            Methods::GetLeaderNondetResult => "get_leader_nondet_result",
            Methods::PostNondetResult => "post_nondet_result",
            Methods::PostMessage => "post_message",
            Methods::PostEvent => "post_event",
            Methods::ConsumeFuel => "consume_fuel",
            Methods::DeployContract => "deploy_contract",
            Methods::EthCall => "eth_call",
            Methods::EthSend => "eth_send",
            Methods::GetBalance => "get_balance",
        }
    }
}

impl TryFrom<u8> for Methods {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, ()> {
        match value {
            0 => Ok(Methods::GetCalldata),
            1 => Ok(Methods::StorageRead),
            2 => Ok(Methods::StorageWrite),
            3 => Ok(Methods::ConsumeResult),
            4 => Ok(Methods::GetLeaderNondetResult),
            5 => Ok(Methods::PostNondetResult),
            6 => Ok(Methods::PostMessage),
            7 => Ok(Methods::PostEvent),
            8 => Ok(Methods::ConsumeFuel),
            9 => Ok(Methods::DeployContract),
            10 => Ok(Methods::EthCall),
            11 => Ok(Methods::EthSend),
            12 => Ok(Methods::GetBalance),
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
}

#[allow(dead_code)]
impl Errors {
    pub fn str_snake_case(self) -> &'static str {
        match self {
            Errors::Ok => "ok",
            Errors::Absent => "absent",
            Errors::Forbidden => "forbidden",
            Errors::IAmLeader => "i_am_leader",
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
            _ => Err(()),
        }
    }
}
