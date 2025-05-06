use serde::{Deserialize, Serialize};

use crate::{calldata, host};

#[derive(Clone, Deserialize, Serialize, Copy, PartialEq, Eq, Debug)]
pub enum On {
    #[serde(rename = "finalized")]
    Finalized,
    #[serde(rename = "accepted")]
    Accepted,
}

fn storage_type_from_bigint<'de, D>(deserializer: D) -> Result<host::StorageType, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct Visitor;

    impl serde::de::Visitor<'_> for Visitor {
        type Value = host::StorageType;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a number")
        }

        fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            let as_u8: u8 = v.try_into().map_err(|_e| E::custom("out of range"))?;
            host::StorageType::try_from(as_u8).map_err(|_e| E::custom("out of range"))
        }

        fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            let as_u8: u8 = v.try_into().map_err(|_e| E::custom("out of range"))?;
            host::StorageType::try_from(as_u8).map_err(|_e| E::custom("out of range"))
        }
    }

    deserializer.deserialize_any(Visitor)
}

#[allow(clippy::enum_variant_names)]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Message {
    EthSend {
        address: calldata::Address,
        #[serde(with = "serde_bytes")]
        calldata: Vec<u8>,
        value: primitive_types::U256,
    },
    EthCall {
        address: calldata::Address,
        #[serde(with = "serde_bytes")]
        calldata: Vec<u8>,
    },
    CallContract {
        address: calldata::Address,
        calldata: calldata::Value,
        #[serde(deserialize_with = "storage_type_from_bigint")]
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
        #[serde(with = "serde_bytes")]
        code: Vec<u8>,
        value: primitive_types::U256,
        on: On,
        salt_nonce: primitive_types::U256,
    },

    RunNondet {
        #[serde(with = "serde_bytes")]
        data_leader: Vec<u8>,
        #[serde(with = "serde_bytes")]
        data_validator: Vec<u8>,
    },

    Sandbox {
        #[serde(with = "serde_bytes")]
        data: Vec<u8>,

        allow_write_ops: bool,
    },

    WebRender(genvm_modules_interfaces::web::RenderPayload),
    ExecPrompt(genvm_modules_interfaces::llm::PromptPayload),
    ExecPromptTemplate(genvm_modules_interfaces::llm::PromptTemplatePayload),

    Rollback(String),
    Return(calldata::Value),
}
