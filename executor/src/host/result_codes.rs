#[derive(PartialEq)]
#[repr(u8)]
pub enum ResultCode {
    Return = 0,
    Rollback = 1,
    None = 2,
    Error = 3,
    ContractError = 4,
}

impl TryFrom<u8> for ResultCode {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, ()> {
        match value {
            0 => Ok(ResultCode::Return),
            1 => Ok(ResultCode::Rollback),
            2 => Ok(ResultCode::None),
            3 => Ok(ResultCode::Error),
            4 => Ok(ResultCode::ContractError),
            _ => Err(()),
        }
    }
}
