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
        address: calldata::Address,
        calldata: Vec<u8>,
        value: primitive_types::U256,
    },
    EthCall {
        address: calldata::Address,
        calldata: Vec<u8>,
    },
    CallContract {
        address: calldata::Address,
        calldata: calldata::Value,
        state: host::StorageType,
    },
    PostMessage {
        address: calldata::Address,
        calldata: calldata::Value,
        value: primitive_types::U256,
        on: On,
    },
    DeployContract {
        calldata: calldata::Value,
        code: Vec<u8>,
        value: primitive_types::U256,
        on: On,
        salt_nonce: primitive_types::U256,
    },

    RunNondet {
        data_leader: Vec<u8>,
        data_validator: Vec<u8>,
    },

    Sandbox {
        data: Vec<u8>,
    },

    WebRender(genvm_modules_interfaces::web::RenderPayload),
    ExecPrompt(genvm_modules_interfaces::llm::PromptPayload),
    ExecPromptTemplate(genvm_modules_interfaces::llm::PromptTemplatePayload),

    Rollback(String),
    Return(calldata::Value),
}
