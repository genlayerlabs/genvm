use crate::calldata;

pub enum Message {
    EthSend,
    EthCall,
    CallContract(),
    PostMessage,
    DeployContract,

    WebRender(),
    ExecPrompt(),
    ExecPromptTemplate(),

    Rollback(String),
    Return(calldata::Value),
}
