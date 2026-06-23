use std::collections::BTreeMap;

use serde_derive::{Deserialize, Serialize};

use genlayer_calldata::codec::{
    Decode, DecodeError as CalldataError, Deserializer, Encode, MapAccess, SeqAccess, Visitor,
};
use genlayer_calldata::{Encoder, Writer};

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

// ── Manual Encode for GenericValue ──────────────────────────────────

impl<W: Writer> Encode<W> for GenericValue {
    type Error = W::Error;

    fn encode(&self, enc: &mut Encoder<W>) -> std::result::Result<(), Self::Error> {
        match self {
            GenericValue::Null => enc.push_null(),
            GenericValue::Bool(v) => enc.push_bool(*v),
            GenericValue::Str(s) => enc.push_str(s),
            GenericValue::Bytes(b) => enc.push_bytes(b),
            GenericValue::Number(f) => {
                let i = *f as i64;
                assert!(
                    i as f64 == *f,
                    "GenericValue::Number({f}) is not an exact integer, cannot encode as bigint"
                );
                enc.push_i64(i)
            }
            GenericValue::Map(map) => {
                enc.start_map(map.len() as u64)?;
                for (k, v) in map {
                    enc.push_map_k(k)?;
                    v.encode(enc)?;
                }
                Ok(())
            }
            GenericValue::Array(arr) => {
                enc.start_array(arr.len() as u64)?;
                for item in arr {
                    item.encode(enc)?;
                }
                Ok(())
            }
        }
    }
}

// ── Manual Decode for GenericValue ──────────────────────────────────

impl Decode for GenericValue {
    fn decode<D: Deserializer>(deserializer: D) -> std::result::Result<Self, CalldataError> {
        struct V;
        impl Visitor for V {
            type Value = GenericValue;

            fn visit_null(self) -> std::result::Result<GenericValue, CalldataError> {
                Ok(GenericValue::Null)
            }

            fn visit_bool(self, value: bool) -> std::result::Result<GenericValue, CalldataError> {
                Ok(GenericValue::Bool(value))
            }

            fn visit_str(self, value: &str) -> std::result::Result<GenericValue, CalldataError> {
                Ok(GenericValue::Str(value.to_owned()))
            }

            fn visit_bytes(self, value: &[u8]) -> std::result::Result<GenericValue, CalldataError> {
                Ok(GenericValue::Bytes(value.to_vec()))
            }

            fn visit_bigint(
                self,
                value: &num_bigint::BigInt,
            ) -> std::result::Result<GenericValue, CalldataError> {
                use num_traits::ToPrimitive;
                let f = value.to_f64().ok_or_else(|| {
                    CalldataError::Custom(format!("bigint {value} cannot be represented as f64"))
                })?;
                Ok(GenericValue::Number(f))
            }

            fn visit_bigint_owned(
                self,
                value: num_bigint::BigInt,
            ) -> std::result::Result<GenericValue, CalldataError> {
                self.visit_bigint(&value)
            }

            fn visit_i64(self, value: i64) -> std::result::Result<GenericValue, CalldataError> {
                Ok(GenericValue::Number(value as f64))
            }

            fn visit_u64(self, value: u64) -> std::result::Result<GenericValue, CalldataError> {
                Ok(GenericValue::Number(value as f64))
            }

            fn visit_seq<A: SeqAccess>(
                self,
                len: u64,
                mut seq: A,
            ) -> std::result::Result<GenericValue, CalldataError> {
                let mut result = Vec::with_capacity(len as usize);
                while let Some(elem) = seq.next_element::<GenericValue>()? {
                    result.push(elem);
                }
                Ok(GenericValue::Array(result))
            }

            fn visit_map<A: MapAccess>(
                self,
                _len: u64,
                mut map: A,
            ) -> std::result::Result<GenericValue, CalldataError> {
                let mut result = BTreeMap::new();
                while let Some((key, value)) = map.next_element::<GenericValue>()? {
                    result.insert(key.to_owned(), value);
                }
                Ok(GenericValue::Map(result))
            }
        }
        deserializer.deserialize(V)
    }
}

// ── Encode / Decode for Result<T> ─────────────────────────────────

#[derive(Clone, Deserialize, Serialize)]
pub enum Result<T> {
    Ok(T),
    UserError(GenericValue),
    FatalError(String),
}

// Externally tagged enum: {"Ok": T} | {"UserError": GenericValue} | {"FatalError": String}
impl<W: Writer, T: Encode<W, Error = W::Error>> Encode<W> for Result<T> {
    type Error = W::Error;

    fn encode(&self, enc: &mut Encoder<W>) -> std::result::Result<(), Self::Error> {
        enc.start_map(1)?;
        match self {
            Result::Ok(v) => {
                enc.push_map_k("Ok")?;
                v.encode(enc)?;
            }
            Result::UserError(v) => {
                enc.push_map_k("UserError")?;
                v.encode(enc)?;
            }
            Result::FatalError(s) => {
                enc.push_map_k("FatalError")?;
                enc.push_str(s)?;
            }
        }
        Ok(())
    }
}

// Externally tagged enum: "Ok" or {"Ok": T}, {"UserError": GenericValue}, {"FatalError": String}
impl<T: Decode> Decode for Result<T> {
    fn decode<D: Deserializer>(deserializer: D) -> std::result::Result<Self, CalldataError> {
        struct V<T>(std::marker::PhantomData<T>);
        impl<T: Decode> Visitor for V<T> {
            type Value = Result<T>;

            fn visit_map<A: MapAccess>(
                self,
                _len: u64,
                mut map: A,
            ) -> std::result::Result<Result<T>, CalldataError> {
                let (key, val) = map.next_element::<genlayer_calldata::Value>()?.ok_or(
                    CalldataError::Custom("expected single-key map for Result enum variant".into()),
                )?;
                match key {
                    "FatalError" => {
                        let inner =
                            String::decode(genlayer_calldata::codec::ValueDeserializer(val))?;
                        Ok(Result::FatalError(inner))
                    }
                    "Ok" => {
                        let inner = T::decode(genlayer_calldata::codec::ValueDeserializer(val))?;
                        Ok(Result::Ok(inner))
                    }
                    "UserError" => {
                        let inner =
                            GenericValue::decode(genlayer_calldata::codec::ValueDeserializer(val))?;
                        Ok(Result::UserError(inner))
                    }
                    _ => Err(CalldataError::Custom(format!(
                        "unknown variant `{key}`, expected one of: FatalError, Ok, UserError"
                    ))),
                }
            }
        }
        deserializer.deserialize(V(std::marker::PhantomData))
    }
}

pub mod llm {
    use std::collections::BTreeMap;

    use serde_derive::{Deserialize, Serialize};

    pub use genlayer_sdk::abi::gl_call::llm_iface::{
        OutputFormat, PromptEqComparativePayload, PromptEqNonComparativeLeaderPayload,
        PromptEqNonComparativeValidatorPayload, PromptPayload, PromptTemplatePayload,
    };

    #[derive(Serialize, Deserialize, genlayer_calldata::Encode, genlayer_calldata::Decode)]
    pub enum Message {
        Prompt {
            payload: PromptPayload,
            remaining_fuel_as_gen: primitive_types::U256,
        },
        PromptTemplate {
            payload: PromptTemplatePayload,
            remaining_fuel_as_gen: primitive_types::U256,
        },
    }

    #[derive(
        Serialize,
        Deserialize,
        Debug,
        PartialEq,
        Eq,
        genlayer_calldata::Encode,
        genlayer_calldata::Decode,
    )]
    #[serde(untagged)]
    #[calldata(untagged)]
    pub enum PromptAnswerData {
        Text(String),
        Object(BTreeMap<String, genlayer_calldata::Value>),
        Bool(bool),
    }

    #[derive(
        Serialize,
        Deserialize,
        Debug,
        PartialEq,
        Eq,
        genlayer_calldata::Encode,
        genlayer_calldata::Decode,
    )]
    pub struct PromptAnswer {
        pub data: PromptAnswerData,
        pub consumed_gen: primitive_types::U256,
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

    #[derive(Serialize, Deserialize, genlayer_calldata::Encode, genlayer_calldata::Decode)]
    pub enum Message {
        Render(RenderPayload, u32),
        Request(RequestPayload, u32),
    }

    #[derive(Serialize, Deserialize, genlayer_calldata::Encode, genlayer_calldata::Decode)]
    pub enum RenderAnswer {
        #[serde(rename = "response")]
        #[calldata(rename = "response")]
        Response(Response),
        #[serde(rename = "text")]
        #[calldata(rename = "text")]
        Text(String),
        #[serde(rename = "image", with = "serde_bytes")]
        #[calldata(
            rename = "image",
            serialize_with = ::genlayer_calldata::codec::as_bytes::serialize,
            deserialize_with = ::genlayer_calldata::codec::as_bytes::deserialize,
        )]
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
    Clone,
    Debug,
    PartialEq,
    Eq,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    Copy,
    PartialOrd,
    Ord,
    genlayer_calldata::Encode,
    genlayer_calldata::Decode,
)]
pub struct GenVMId(pub u64);

impl std::fmt::Display for GenVMId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    genlayer_calldata::Encode,
    genlayer_calldata::Decode,
)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Leader,
    Validator,
}

#[derive(Debug, Serialize, Deserialize, genlayer_calldata::Encode, genlayer_calldata::Decode)]
pub struct GenVMHello {
    pub genvm_id: GenVMId,
    pub role: Role,
    #[calldata(deserialize_with = decode_host_data)]
    pub host_data: HostData,
    #[serde(default)]
    pub gas_data: std::collections::BTreeMap<String, String>,
    pub initial_time_units_allocation: u32,
}

/// Decode a `serde_json::Value` from a calldata `Value`.
fn calldata_to_json(
    val: genlayer_calldata::Value,
) -> std::result::Result<serde_json::Value, CalldataError> {
    match val {
        genlayer_calldata::Value::Null => Ok(serde_json::Value::Null),
        genlayer_calldata::Value::Bool(b) => Ok(serde_json::Value::Bool(b)),
        genlayer_calldata::Value::Number(n) => {
            use num_traits::ToPrimitive;
            let i = n.to_i64().ok_or_else(|| {
                CalldataError::Custom(format!("bigint {n} cannot be represented as i64"))
            })?;
            Ok(serde_json::Value::Number(serde_json::Number::from(i)))
        }
        genlayer_calldata::Value::Str(s) => Ok(serde_json::Value::String(s)),
        genlayer_calldata::Value::Bytes(_) => Err(CalldataError::Custom(
            "cannot convert calldata bytes to serde_json::Value".into(),
        )),
        genlayer_calldata::Value::Address(_) => Err(CalldataError::Custom(
            "cannot convert calldata address to serde_json::Value".into(),
        )),
        genlayer_calldata::Value::Array(arr) => {
            let mut out = Vec::with_capacity(arr.len());
            for v in arr {
                out.push(calldata_to_json(v)?);
            }
            Ok(serde_json::Value::Array(out))
        }
        genlayer_calldata::Value::Map(map) => {
            let mut out = serde_json::Map::new();
            for (k, v) in map {
                out.insert(k, calldata_to_json(v)?);
            }
            Ok(serde_json::Value::Object(out))
        }
    }
}

/// Custom deserializer for HostData from calldata. HostData is encoded as a map
/// with known fields `node_address` and `tx_id`; everything else goes into `rest`.
fn decode_host_data(val: genlayer_calldata::Value) -> std::result::Result<HostData, CalldataError> {
    let genlayer_calldata::Value::Map(mut map) = val else {
        return Err(CalldataError::Unexpected("expected map for HostData"));
    };

    let node_address = map
        .remove("node_address")
        .ok_or(CalldataError::Custom("missing field `node_address`".into()))
        .and_then(|v| match v {
            genlayer_calldata::Value::Str(s) => Ok(s),
            _ => Err(CalldataError::Unexpected(
                "expected string for node_address",
            )),
        })?;

    let tx_id = map
        .remove("tx_id")
        .ok_or(CalldataError::Custom("missing field `tx_id`".into()))
        .and_then(|v| match v {
            genlayer_calldata::Value::Str(s) => Ok(s),
            _ => Err(CalldataError::Unexpected("expected string for tx_id")),
        })?;

    let mut rest = serde_json::Map::new();
    for (k, v) in map {
        rest.insert(k, calldata_to_json(v)?);
    }

    Ok(HostData {
        node_address,
        tx_id,
        rest,
    })
}

// ── Manual Encode for HostData (flattened serde_json fields) ───────

/// Helper: encode a serde_json::Value into calldata format.
fn encode_json_value<W: Writer>(
    value: &serde_json::Value,
    enc: &mut Encoder<W>,
) -> std::result::Result<(), W::Error> {
    match value {
        serde_json::Value::Null => enc.push_null(),
        serde_json::Value::Bool(b) => enc.push_bool(*b),
        serde_json::Value::Number(n) => {
            // JSON numbers: try i64, then u64, then error (no f64 in calldata)
            if let Some(i) = n.as_i64() {
                enc.push_i64(i)
            } else if let Some(u) = n.as_u64() {
                enc.push_u64(u)
            } else {
                panic!(
                    "serde_json::Number({n}) cannot be represented as integer for calldata encoding"
                );
            }
        }
        serde_json::Value::String(s) => enc.push_str(s),
        serde_json::Value::Array(arr) => {
            enc.start_array(arr.len() as u64)?;
            for item in arr {
                encode_json_value(item, enc)?;
            }
            Ok(())
        }
        serde_json::Value::Object(map) => {
            // serde_json::Map iteration order may not be sorted; collect and sort
            let mut entries: Vec<_> = map.iter().collect();
            entries.sort_by_key(|(k, _)| k.as_str());
            enc.start_map(entries.len() as u64)?;
            for (k, v) in entries {
                enc.push_map_k(k)?;
                encode_json_value(v, enc)?;
            }
            Ok(())
        }
    }
}

impl<W: Writer> Encode<W> for HostData {
    type Error = W::Error;

    fn encode(&self, enc: &mut Encoder<W>) -> std::result::Result<(), Self::Error> {
        // Collect all keys: the two named fields + all keys from `rest`.
        // Must be emitted in sorted order for the calldata map format.
        let mut entries: Vec<(&str, Option<&serde_json::Value>)> = Vec::new();

        // Sentinel: None means it's one of the named fields
        entries.push(("node_address", None));
        entries.push(("tx_id", None));

        for (k, v) in &self.rest {
            entries.push((k.as_str(), Some(v)));
        }

        entries.sort_by_key(|(k, _)| *k);

        enc.start_map(entries.len() as u64)?;
        for (key, json_val) in &entries {
            enc.push_map_k(key)?;
            match (key, json_val) {
                (&"node_address", None) => enc.push_str(&self.node_address)?,
                (&"tx_id", None) => enc.push_str(&self.tx_id)?,
                (_, Some(v)) => encode_json_value(v, enc)?,
                _ => unreachable!(),
            }
        }
        Ok(())
    }
}
