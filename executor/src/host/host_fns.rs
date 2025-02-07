use serde_derive::Serialize;
#[derive(PartialEq, Clone, Copy, Serialize)]
#[repr(u8)]
pub enum Methods {
    GetCalldata = 0,
    GetCode = 1,
    StorageRead = 2,
    StorageWrite = 3,
    ConsumeResult = 4,
    GetLeaderNondetResult = 5,
    PostNondetResult = 6,
    PostMessage = 7,
    ConsumeFuel = 8,
    DeployContract = 9,
    EthCall = 10,
    EthSend = 11,
    GetBalance = 12,
}

impl TryFrom<u8> for Methods {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, ()> {
        match value {
            0 => Ok(Methods::GetCalldata),
            1 => Ok(Methods::GetCode),
            2 => Ok(Methods::StorageRead),
            3 => Ok(Methods::StorageWrite),
            4 => Ok(Methods::ConsumeResult),
            5 => Ok(Methods::GetLeaderNondetResult),
            6 => Ok(Methods::PostNondetResult),
            7 => Ok(Methods::PostMessage),
            8 => Ok(Methods::ConsumeFuel),
            9 => Ok(Methods::DeployContract),
            10 => Ok(Methods::EthCall),
            11 => Ok(Methods::EthSend),
            12 => Ok(Methods::GetBalance),
            _ => Err(()),
        }
    }
}
