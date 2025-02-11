use serde_derive::Serialize;
#[derive(PartialEq, Clone, Copy, Serialize)]
#[repr(u8)]
pub enum ResultCode {
    Return = 0,
    Rollback = 1,
    ContractError = 2,
    Error = 3,
    None = 4,
    NoLeaders = 5,
}

impl TryFrom<u8> for ResultCode {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, ()> {
        match value {
            0 => Ok(ResultCode::Return),
            1 => Ok(ResultCode::Rollback),
            2 => Ok(ResultCode::ContractError),
            3 => Ok(ResultCode::Error),
            4 => Ok(ResultCode::None),
            5 => Ok(ResultCode::NoLeaders),
            _ => Err(()),
        }
    }
}
#[derive(PartialEq, Clone, Copy, Serialize)]
#[repr(u8)]
pub enum StorageType {
    Default = 0,
    LatestFinal = 1,
    LatestNonFinal = 2,
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
