//! Message types for gl_call operations.
//!
//! This module defines the payload structures for all gl_call operations,
//! including web requests, LLM prompts, contract calls, and more.

use std::collections::BTreeMap;

use bytes::Bytes;
use serde::{Deserialize, Serialize};

use crate::calldata;

use super::consts as public_abi;

/// Web module interface types for WebRender and WebRequest operations.
pub mod web_iface {
    use serde::{Deserialize, Serialize};
    use std::collections::BTreeMap;

    /// Render mode for WebRender operations.
    #[derive(Serialize, Deserialize)]
    pub enum RenderMode {
        #[serde(rename = "text")]
        Text,
        #[serde(rename = "html")]
        HTML,
        #[serde(rename = "screenshot")]
        Screenshot,
    }

    /// Duration to wait after page load before capturing content.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum WaitAfterLoaded {
        Seconds(u64),
        Millis(u64),
    }

    impl WaitAfterLoaded {
        pub fn as_secs_f64(&self) -> f64 {
            match self {
                WaitAfterLoaded::Seconds(s) => *s as f64,
                WaitAfterLoaded::Millis(ms) => *ms as f64 / 1000.0,
            }
        }
    }

    struct WaitAfterLoadedVisitor;

    impl serde::de::Visitor<'_> for WaitAfterLoadedVisitor {
        type Value = WaitAfterLoaded;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("expected string | null")
        }

        fn visit_none<E>(self) -> std::result::Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(WaitAfterLoaded::Millis(0))
        }

        fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            if let Some(ms_str) = value.strip_suffix("ms") {
                let millis = ms_str.parse::<u64>().map_err(E::custom)?;
                Ok(WaitAfterLoaded::Millis(millis))
            } else if let Some(secs_str) = value.strip_suffix("s") {
                let seconds = secs_str.parse::<u64>().map_err(E::custom)?;
                Ok(WaitAfterLoaded::Seconds(seconds))
            } else {
                Err(E::invalid_value(
                    serde::de::Unexpected::Str(value),
                    &"expected a string ending with 's' or 'ms'",
                ))
            }
        }
    }

    impl<'de> serde::Deserialize<'de> for WaitAfterLoaded {
        fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            deserializer.deserialize_str(WaitAfterLoadedVisitor)
        }
    }

    impl serde::Serialize for WaitAfterLoaded {
        fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            match self {
                WaitAfterLoaded::Seconds(v) => {
                    let as_str = format!("{}s", v);
                    serializer.serialize_str(&as_str)
                }
                WaitAfterLoaded::Millis(v) => {
                    let as_str = format!("{}ms", v);
                    serializer.serialize_str(&as_str)
                }
            }
        }
    }

    /// Payload for WebRender operations.
    #[derive(Serialize, Deserialize)]
    pub struct RenderPayload {
        pub mode: RenderMode,
        pub url: String,
        pub wait_after_loaded: WaitAfterLoaded,
    }

    /// HTTP request method for WebRequest operations.
    #[derive(Debug, Serialize, Deserialize)]
    pub enum RequestMethod {
        GET,
        POST,
        HEAD,
        DELETE,
        OPTIONS,
        PATCH,
    }

    /// HTTP response from WebRequest or WebRender operations.
    #[derive(Debug, Serialize, Deserialize)]
    pub struct Response {
        pub status: u16,
        pub headers: BTreeMap<String, bytes::Bytes>,

        #[serde(with = "serde_bytes")]
        pub body: Vec<u8>,
    }

    fn default_none<T>() -> Option<T> {
        None
    }

    fn default_false() -> bool {
        false
    }

    /// Payload for WebRequest operations.
    #[derive(Serialize, Deserialize)]
    pub struct RequestPayload {
        pub method: RequestMethod,
        pub url: String,
        pub headers: BTreeMap<String, bytes::Bytes>,

        #[serde(with = "serde_bytes", default = "default_none")]
        pub body: Option<Vec<u8>>,
        #[serde(default = "default_false")]
        pub sign: bool,
    }
}

/// LLM module interface types for ExecPrompt and ExecPromptTemplate operations.
pub mod llm_iface {
    use serde::{Deserialize, Serialize};

    /// Output format for LLM prompt responses.
    #[derive(Clone, Deserialize, Serialize, Copy, PartialEq, Eq, Debug)]
    pub enum OutputFormat {
        #[serde(rename = "text")]
        Text,
        #[serde(rename = "json")]
        JSON,
    }

    /// Variables for EqComparative prompt template.
    #[derive(Serialize, Deserialize)]
    pub struct PromptIDVarsComparative {
        pub leader_answer: String,
        pub validator_answer: String,
        pub principle: String,
    }

    /// Variables for EqNonComparativeValidator prompt template.
    #[derive(Serialize, Deserialize)]
    pub struct PromptIDVarsNonComparativeValidator {
        pub task: String,
        pub criteria: String,
        pub input: String,
        pub output: String,
    }

    /// Variables for EqNonComparativeLeader prompt template.
    #[derive(Serialize, Deserialize)]
    pub struct PromptIDVarsNonComparativeLeader {
        pub task: String,
        pub criteria: String,
        pub input: String,
    }

    fn default_text() -> OutputFormat {
        OutputFormat::Text
    }

    /// Payload for ExecPrompt operations.
    #[derive(Serialize, Deserialize, Debug)]
    pub struct PromptPayload {
        #[serde(default = "default_text")]
        pub response_format: OutputFormat,
        pub prompt: String,
        pub images: Vec<bytes::Bytes>,
    }

    #[derive(Serialize, Deserialize)]
    pub struct PromptEqComparativePayload {
        #[serde(flatten)]
        pub vars: PromptIDVarsComparative,
    }

    #[derive(Serialize, Deserialize)]
    pub struct PromptEqNonComparativeValidatorPayload {
        #[serde(flatten)]
        pub vars: PromptIDVarsNonComparativeValidator,
    }

    #[derive(Serialize, Deserialize)]
    pub struct PromptEqNonComparativeLeaderPayload {
        #[serde(flatten)]
        pub vars: PromptIDVarsNonComparativeLeader,
    }

    /// Payload for ExecPromptTemplate operations.
    #[derive(Serialize, Deserialize)]
    #[serde(tag = "template")]
    pub enum PromptTemplatePayload {
        EqComparative(PromptEqComparativePayload),
        EqNonComparativeValidator(PromptEqNonComparativeValidatorPayload),
        EqNonComparativeLeader(PromptEqNonComparativeLeaderPayload),
    }
}

/// When to execute a posted message or deploy a contract.
#[derive(Clone, Deserialize, Serialize, Copy, PartialEq, Eq, Debug)]
pub enum On {
    #[serde(rename = "finalized")]
    Finalized,
    #[serde(rename = "accepted")]
    Accepted,
}

fn storage_type_from_bigint<'de, D>(deserializer: D) -> Result<public_abi::StorageType, D::Error>
where
    D: serde::Deserializer<'de>,
{
    struct Visitor;

    impl serde::de::Visitor<'_> for Visitor {
        type Value = public_abi::StorageType;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a number")
        }

        fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            let as_u8: u8 = v.try_into().map_err(|_e| E::custom("out of range"))?;
            public_abi::StorageType::try_from(as_u8).map_err(|_e| E::custom("out of range"))
        }

        fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            let as_u8: u8 = v.try_into().map_err(|_e| E::custom("out of range"))?;
            public_abi::StorageType::try_from(as_u8).map_err(|_e| E::custom("out of range"))
        }
    }

    deserializer.deserialize_any(Visitor)
}

/// Payload for Trace operations.
#[derive(Clone, Deserialize, Serialize, Debug)]
pub enum TracePayload {
    /// Log a debug message with timing information.
    Message(String),
    /// Get elapsed execution time in microseconds.
    RuntimeMicroSec,
}

/// All available gl_call message types.
///
/// Each variant corresponds to a specific blockchain operation that can be
/// invoked via the [`super::wasi::gl_call`] function.
#[allow(clippy::enum_variant_names)]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Message {
    EthCall {
        address: calldata::Address,
        calldata: Bytes,
    },
    CallContract {
        address: calldata::Address,
        calldata: calldata::Value,
        #[serde(deserialize_with = "storage_type_from_bigint")]
        state: public_abi::StorageType,
    },

    EthSend {
        address: calldata::Address,
        calldata: Bytes,
        value: primitive_types::U256,
    },
    PostMessage {
        address: calldata::Address,
        calldata: calldata::Value,
        value: primitive_types::U256,
        on: On,
    },
    DeployContract {
        calldata: calldata::Value,
        code: Bytes,
        value: primitive_types::U256,
        on: On,
        salt_nonce: primitive_types::U256,
    },
    EmitEvent {
        topics: Vec<Bytes>,
        blob: BTreeMap<String, calldata::Value>,
    },

    RunNondet {
        data_leader: Bytes,
        data_validator: Bytes,
    },

    Sandbox {
        data: Bytes,

        allow_write_ops: bool,
    },

    WebRender(web_iface::RenderPayload),
    WebRequest(web_iface::RequestPayload),
    ExecPrompt(llm_iface::PromptPayload),
    ExecPromptTemplate(llm_iface::PromptTemplatePayload),

    Rollback(calldata::Value),
    Return(calldata::Value),

    Trace(TracePayload),
}
