use std::collections::BTreeMap;

use serde_derive::{Deserialize, Serialize};

pub trait Web {
    fn get_webpage(
        &self,
        config: String,
        url: String,
    ) -> tokio::task::JoinHandle<anyhow::Result<Box<[u8]>>>;
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum GenericValue {
    Null,
    Bool(bool),
    Str(String),
    Bytes(#[serde(with = "serde_bytes")] Vec<u8>),
    Number(f64),
    Map(BTreeMap<String, GenericValue>),
    Array(Vec<GenericValue>),
}

impl From<String> for GenericValue {
    fn from(value: String) -> Self {
        GenericValue::Str(value)
    }
}

impl From<i32> for GenericValue {
    fn from(value: i32) -> Self {
        GenericValue::Number(value as f64)
    }
}

impl From<u16> for GenericValue {
    fn from(value: u16) -> Self {
        GenericValue::Number(value as f64)
    }
}

impl From<f64> for GenericValue {
    fn from(value: f64) -> Self {
        GenericValue::Number(value)
    }
}

impl From<u32> for GenericValue {
    fn from(value: u32) -> Self {
        GenericValue::Number(value as f64)
    }
}

impl From<bool> for GenericValue {
    fn from(value: bool) -> Self {
        GenericValue::Bool(value)
    }
}

impl From<Vec<u8>> for GenericValue {
    fn from(value: Vec<u8>) -> Self {
        GenericValue::Bytes(value)
    }
}

impl From<serde_json::Value> for GenericValue {
    fn from(value: serde_json::Value) -> Self {
        match value {
            serde_json::Value::Null => GenericValue::Null,
            serde_json::Value::Bool(x) => GenericValue::Bool(x),
            serde_json::Value::Number(number) => GenericValue::Number(number.as_f64().unwrap()),
            serde_json::Value::String(s) => GenericValue::Str(s),
            serde_json::Value::Array(values) => {
                GenericValue::Array(values.into_iter().map(Into::into).collect())
            }
            serde_json::Value::Object(map) => GenericValue::Map(BTreeMap::from_iter(
                map.into_iter().map(|(k, v)| (k, v.into())),
            )),
        }
    }
}

impl GenericValue {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            GenericValue::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_num(&self) -> Option<f64> {
        match self {
            GenericValue::Number(s) => Some(*s),
            _ => None,
        }
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub enum Result<T> {
    Ok(T),
    UserError(GenericValue),
    FatalError(String),
}

pub mod llm {
    use serde_derive::{Deserialize, Serialize};

    pub use genlayer_sdk::abi::gl_call::llm_iface::{
        OutputFormat, PromptEqComparativePayload, PromptEqNonComparativeLeaderPayload,
        PromptEqNonComparativeValidatorPayload, PromptIDVarsComparative,
        PromptIDVarsNonComparativeLeader, PromptIDVarsNonComparativeValidator, PromptPayload,
        PromptTemplatePayload,
    };

    #[derive(Serialize, Deserialize)]
    pub enum Message {
        Prompt {
            payload: PromptPayload,
            remaining_fuel_as_gen: u64,
        },
        PromptTemplate {
            payload: PromptTemplatePayload,
            remaining_fuel_as_gen: u64,
        },

        GetStats,
    }

    #[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
    #[serde(untagged)]
    pub enum PromptAnswerData {
        Text(String),
        Bool(bool),
    }

    #[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
    pub struct PromptAnswer {
        pub data: PromptAnswerData,
        pub consumed_gen: u64,
    }

    impl PromptAnswer {
        pub fn map_text(&mut self, f: impl FnOnce(&mut String)) {
            if let PromptAnswerData::Text(t) = &mut self.data {
                f(t)
            }
        }
    }
}

pub mod web {
    use serde_derive::{Deserialize, Serialize};

    pub use genlayer_sdk::abi::gl_call::web_iface::{
        RenderPayload, RequestMethod, RequestPayload, Response,
    };

    #[derive(Serialize, Deserialize)]
    pub enum Message {
        Render(RenderPayload, u32),
        Request(RequestPayload, u32),
        GetStats,
    }

    #[derive(Serialize, Deserialize)]
    pub enum RenderAnswer {
        #[serde(rename = "response")]
        Response(Response),
        #[serde(rename = "text")]
        Text(String),
        #[serde(rename = "image", with = "serde_bytes")]
        Image(Vec<u8>),
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HostData {
    pub node_address: String,
    pub tx_id: String,
    #[serde(flatten)]
    pub rest: serde_json::Map<String, serde_json::Value>,
}

#[derive(
    Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, Copy, PartialOrd, Ord,
)]
pub struct GenVMId(pub u64);

impl std::fmt::Display for GenVMId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GenVMHello {
    pub genvm_id: GenVMId,
    pub host_data: HostData,
}
