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
#[derive(PartialEq)]
#[repr(u8)]
pub enum ResultCode {
    Return = 0,
    Rollback = 1,
    None = 2,
    Error = 3,
}
