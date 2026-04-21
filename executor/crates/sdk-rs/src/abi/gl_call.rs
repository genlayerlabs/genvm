//! Message types for gl_call operations.
//!
//! This module defines the payload structures for all gl_call operations,
//! including web requests, LLM prompts, contract calls, and more.

use std::collections::BTreeMap;

use bytes::Bytes;
use genlayer_calldata::{Decode, Encode};
use serde::{Deserialize, Serialize};

use crate::calldata;

use super::consts as public_abi;

/// Web module interface types for WebRender and WebRequest operations.
pub mod web_iface {
    use genlayer_calldata::{Decode, Encode};
    use serde::{Deserialize, Serialize};
    use std::collections::BTreeMap;

    /// Render mode for WebRender operations.
    #[derive(Clone, PartialEq, Serialize, Deserialize, Encode, Decode, Debug)]
    #[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
    pub enum RenderMode {
        #[serde(rename = "text")]
        #[calldata(rename = "text")]
        Text,
        #[serde(rename = "html")]
        #[calldata(rename = "html")]
        HTML,
        #[serde(rename = "screenshot")]
        #[calldata(rename = "screenshot")]
        Screenshot,
    }

    /// Duration to wait after page load before capturing content.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    #[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
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

    pub(crate) fn encode_wait_after_loaded<W: genlayer_calldata::Writer>(
        val: &WaitAfterLoaded,
        enc: &mut genlayer_calldata::Encoder<W>,
    ) -> Result<(), W::Error> {
        let s = match val {
            WaitAfterLoaded::Seconds(v) => format!("{v}s"),
            WaitAfterLoaded::Millis(v) => format!("{v}ms"),
        };
        enc.push_str(&s)
    }

    pub(crate) fn decode_wait_after_loaded(
        val: genlayer_calldata::Value,
    ) -> Result<WaitAfterLoaded, genlayer_calldata::codec::Error> {
        let genlayer_calldata::Value::Str(s) = val else {
            return Err(genlayer_calldata::codec::Error::Unexpected(
                "expected string",
            ));
        };
        if let Some(ms_str) = s.strip_suffix("ms") {
            let millis = ms_str
                .parse::<u64>()
                .map_err(|e| genlayer_calldata::codec::Error::Custom(e.to_string()))?;
            Ok(WaitAfterLoaded::Millis(millis))
        } else if let Some(secs_str) = s.strip_suffix("s") {
            let seconds = secs_str
                .parse::<u64>()
                .map_err(|e| genlayer_calldata::codec::Error::Custom(e.to_string()))?;
            Ok(WaitAfterLoaded::Seconds(seconds))
        } else {
            Err(genlayer_calldata::codec::Error::Custom(format!(
                "expected string ending with 's' or 'ms', got '{s}'"
            )))
        }
    }

    fn default_none<T>() -> Option<T> {
        None
    }

    fn default_false() -> bool {
        false
    }

    /// Payload for WebRender operations.
    #[derive(Clone, PartialEq, Serialize, Deserialize, Encode, Decode, Debug)]
    #[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
    pub struct RenderPayload {
        pub mode: RenderMode,
        pub url: String,
        #[calldata(
            serialize_with = encode_wait_after_loaded,
            deserialize_with = decode_wait_after_loaded
        )]
        pub wait_after_loaded: WaitAfterLoaded,
    }

    /// HTTP request method for WebRequest operations.
    #[derive(Clone, PartialEq, Debug, Serialize, Deserialize, Encode, Decode)]
    #[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
    pub enum RequestMethod {
        GET,
        POST,
        HEAD,
        PUT,
        DELETE,
        OPTIONS,
        PATCH,
    }

    /// HTTP response from WebRequest or WebRender operations.
    #[derive(Clone, PartialEq, Debug, Serialize, Deserialize, Encode, Decode)]
    #[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
    pub struct Response {
        pub status: u16,
        #[cfg_attr(feature = "arbitrary", arbitrary(with = crate::abi::arb::arb_btreemap_bytes))]
        pub headers: BTreeMap<String, bytes::Bytes>,

        #[serde(with = "serde_bytes")]
        pub body: Vec<u8>,
    }

    /// Payload for WebRequest operations.
    #[derive(Clone, PartialEq, Serialize, Deserialize, Encode, Decode, Debug)]
    #[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
    pub struct RequestPayload {
        pub method: RequestMethod,
        pub url: String,
        #[cfg_attr(feature = "arbitrary", arbitrary(with = crate::abi::arb::arb_btreemap_bytes))]
        pub headers: BTreeMap<String, bytes::Bytes>,

        #[serde(with = "serde_bytes", default = "default_none")]
        #[calldata(default = default_none)]
        pub body: Option<Vec<u8>>,
        #[serde(default = "default_false")]
        #[calldata(default = default_false)]
        pub sign: bool,
    }
}

/// LLM module interface types for ExecPrompt and ExecPromptTemplate operations.
pub mod llm_iface {
    use genlayer_calldata::{Decode, Encode};
    use serde::{Deserialize, Serialize};

    /// Output format for LLM prompt responses.
    #[derive(Clone, Deserialize, Serialize, Encode, Decode, Copy, PartialEq, Eq, Debug)]
    #[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
    pub enum OutputFormat {
        #[serde(rename = "text")]
        #[calldata(rename = "text")]
        Text,
        #[serde(rename = "json")]
        #[calldata(rename = "json")]
        JSON,
    }

    /// Variables for EqComparative prompt template.
    #[derive(Clone, PartialEq, Serialize, Deserialize, Encode, Decode, Debug)]
    #[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
    pub struct PromptIDVarsComparative {
        pub leader_answer: String,
        pub validator_answer: String,
        pub principle: String,
    }

    /// Variables for EqNonComparativeValidator prompt template.
    #[derive(Clone, PartialEq, Serialize, Deserialize, Encode, Decode, Debug)]
    #[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
    pub struct PromptIDVarsNonComparativeValidator {
        pub task: String,
        pub criteria: String,
        pub input: String,
        pub output: String,
    }

    /// Variables for EqNonComparativeLeader prompt template.
    #[derive(Clone, PartialEq, Serialize, Deserialize, Encode, Decode, Debug)]
    #[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
    pub struct PromptIDVarsNonComparativeLeader {
        pub task: String,
        pub criteria: String,
        pub input: String,
    }

    fn default_text() -> OutputFormat {
        OutputFormat::Text
    }

    /// Payload for ExecPrompt operations.
    #[derive(Clone, PartialEq, Serialize, Deserialize, Encode, Decode, Debug)]
    #[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
    pub struct PromptPayload {
        #[serde(default = "default_text")]
        #[calldata(default = default_text)]
        pub response_format: OutputFormat,
        pub prompt: String,
        #[cfg_attr(feature = "arbitrary", arbitrary(with = crate::abi::arb::arb_vec_bytes))]
        pub images: Vec<bytes::Bytes>,
    }

    #[derive(Clone, PartialEq, Serialize, Deserialize, Encode, Decode, Debug)]
    #[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
    pub struct PromptEqComparativePayload {
        #[serde(flatten)]
        pub vars: PromptIDVarsComparative,
    }

    #[derive(Clone, PartialEq, Serialize, Deserialize, Encode, Decode, Debug)]
    #[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
    pub struct PromptEqNonComparativeValidatorPayload {
        #[serde(flatten)]
        pub vars: PromptIDVarsNonComparativeValidator,
    }

    #[derive(Clone, PartialEq, Serialize, Deserialize, Encode, Decode, Debug)]
    #[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
    pub struct PromptEqNonComparativeLeaderPayload {
        #[serde(flatten)]
        pub vars: PromptIDVarsNonComparativeLeader,
    }

    /// Payload for ExecPromptTemplate operations.
    #[derive(Clone, PartialEq, Serialize, Deserialize, Encode, Decode, Debug)]
    #[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
    #[serde(tag = "template")]
    pub enum PromptTemplatePayload {
        EqComparative(PromptEqComparativePayload),
        EqNonComparativeValidator(PromptEqNonComparativeValidatorPayload),
        EqNonComparativeLeader(PromptEqNonComparativeLeaderPayload),
    }
}

/// When to execute a posted message or deploy a contract.
#[derive(Clone, Deserialize, Serialize, Encode, Decode, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub enum On {
    #[serde(rename = "finalized")]
    #[calldata(rename = "finalized")]
    Finalized,
    #[serde(rename = "accepted")]
    #[calldata(rename = "accepted")]
    Accepted,
}

fn encode_storage_type<W: calldata::Writer>(
    st: &public_abi::StorageType,
    enc: &mut calldata::Encoder<W>,
) -> Result<(), W::Error> {
    enc.push_u64(st.value() as u64)
}

fn decode_storage_type(
    val: calldata::Value,
) -> Result<public_abi::StorageType, calldata::codec::Error> {
    let calldata::Value::Number(n) = val else {
        return Err(calldata::codec::Error::Unexpected("expected number"));
    };
    let v: u8 = <u8 as TryFrom<&num_bigint::BigInt>>::try_from(&n)
        .map_err(|_| calldata::codec::Error::Custom("StorageType out of range".into()))?;
    public_abi::StorageType::try_from(v)
        .map_err(|_| calldata::codec::Error::Custom("invalid StorageType".into()))
}

/// Payload for Trace operations.
#[derive(Clone, PartialEq, Deserialize, Serialize, calldata::Encode, calldata::Decode, Debug)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
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
#[derive(PartialEq, Debug, calldata::Encode, calldata::Decode)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub enum Message {
    EthCall {
        address: calldata::Address,
        #[cfg_attr(feature = "arbitrary", arbitrary(with = crate::abi::arb::arb_bytes))]
        calldata: Bytes,
    },
    CallContract {
        address: calldata::Address,
        calldata: calldata::Value,
        #[calldata(
            serialize_with = encode_storage_type,
            deserialize_with = decode_storage_type
        )]
        state: public_abi::StorageType,
    },

    EthSend {
        address: calldata::Address,
        #[cfg_attr(feature = "arbitrary", arbitrary(with = crate::abi::arb::arb_bytes))]
        calldata: Bytes,
        #[cfg_attr(feature = "arbitrary", arbitrary(with = crate::abi::arb::arb_u256))]
        value: primitive_types::U256,
    },
    PostMessage {
        address: calldata::Address,
        calldata: calldata::Value,
        #[cfg_attr(feature = "arbitrary", arbitrary(with = crate::abi::arb::arb_u256))]
        value: primitive_types::U256,
        on: On,
    },
    DeployContract {
        calldata: calldata::Value,
        #[cfg_attr(feature = "arbitrary", arbitrary(with = crate::abi::arb::arb_bytes))]
        code: Bytes,
        #[cfg_attr(feature = "arbitrary", arbitrary(with = crate::abi::arb::arb_u256))]
        value: primitive_types::U256,
        on: On,
        #[cfg_attr(feature = "arbitrary", arbitrary(with = crate::abi::arb::arb_u256))]
        salt_nonce: primitive_types::U256,
    },
    EmitEvent {
        #[cfg_attr(feature = "arbitrary", arbitrary(with = crate::abi::arb::arb_vec_bytes))]
        topics: Vec<Bytes>,
        blob: BTreeMap<String, calldata::Value>,
    },

    RunNondet {
        #[cfg_attr(feature = "arbitrary", arbitrary(with = crate::abi::arb::arb_bytes))]
        data_leader: Bytes,
        #[cfg_attr(feature = "arbitrary", arbitrary(with = crate::abi::arb::arb_bytes))]
        data_validator: Bytes,
    },

    Sandbox {
        #[cfg_attr(feature = "arbitrary", arbitrary(with = crate::abi::arb::arb_bytes))]
        data: Bytes,

        allow_write_ops: bool,
    },

    WebRender(web_iface::RenderPayload),
    WebRequest(web_iface::RequestPayload),
    ExecPrompt(llm_iface::PromptPayload),
    ExecPromptTemplate(llm_iface::PromptTemplatePayload),

    #[deprecated(note = "Use UserError. Will be removed before 1.0 release")]
    #[allow(deprecated)]
    Rollback(calldata::Value),
    UserError(calldata::Value),
    Return(calldata::Value),

    Trace(TracePayload),
}
