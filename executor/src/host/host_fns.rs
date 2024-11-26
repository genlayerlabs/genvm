#[derive(PartialEq)]
#[repr(u8)]
pub enum Methods {
    AppendCalldata = 0,
    GetCode = 1,
    StorageRead = 2,
    StorageWrite = 3,
    ConsumeResult = 4,
    GetLeaderNondetResult = 5,
    PostNondetResult = 6,
    PostMessage = 7,
    ConsumeFuel = 8,
}

impl TryFrom<u8> for Methods {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, ()> {
        match value {
            0 => Ok(Methods::AppendCalldata),
            1 => Ok(Methods::GetCode),
            2 => Ok(Methods::StorageRead),
            3 => Ok(Methods::StorageWrite),
            4 => Ok(Methods::ConsumeResult),
            5 => Ok(Methods::GetLeaderNondetResult),
            6 => Ok(Methods::PostNondetResult),
            7 => Ok(Methods::PostMessage),
            8 => Ok(Methods::ConsumeFuel),
            _ => Err(()),
        }
    }
}
#[derive(PartialEq)]
#[repr(u8)]
pub enum ResultCode {
    Return = 0,
    Rollback = 1,
    None = 2,
    Error = 3,
}

impl TryFrom<u8> for ResultCode {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, ()> {
        match value {
            0 => Ok(ResultCode::Return),
            1 => Ok(ResultCode::Rollback),
            2 => Ok(ResultCode::None),
            3 => Ok(ResultCode::Error),
            _ => Err(()),
        }
    }
}
