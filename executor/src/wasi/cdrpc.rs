use serde::{Deserialize, Serialize};

use crate::{calldata, host};

#[derive(Clone, Deserialize, Serialize, Copy, PartialEq, Eq, Debug)]
pub enum On {
    #[serde(rename = "Finalized")]
    Finalized,
    #[serde(rename = "Accepted")]
    Accepted,
}

#[derive(Deserialize)]
pub enum Message {
    EthSend {
        address: (),
        calldata: calldata::Value,
        value: (),
    },
    EthCall {
        address: (),
        calldata: calldata::Value,
    },
    CallContract {
        address: (),
        calldata: calldata::Value,
        state: host::StorageType,
    },
    PostMessage {
        address: (),
        calldata: calldata::Value,
        value: (),
        on: On,
    },
    DeployContract {
        calldata: calldata::Value,
        code: Vec<u8>,
        value: (),
        on: On,
        salt_nonce: (),
    },

    WebRender(genvm_modules_interfaces::web::RenderPayload),
    ExecPrompt(genvm_modules_interfaces::llm::PromptPayload),
    ExecPromptTemplate(genvm_modules_interfaces::llm::PromptTemplatePayload),

    Rollback(String),
    Return(calldata::Value),
}
