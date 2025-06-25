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

pub struct ParsedDuration(pub tokio::time::Duration);

struct ParsedDurationVisitor;

impl serde::de::Visitor<'_> for ParsedDurationVisitor {
    type Value = ParsedDuration;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("expected string | null")
    }

    fn visit_none<E>(self) -> std::result::Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(ParsedDuration(tokio::time::Duration::ZERO))
    }

    fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let re = regex::Regex::new(r#"^(\d+)(m?s)$"#).unwrap();
        let caps = re
            .captures(value)
            .ok_or(E::custom("invalid duration format"))?;

        let int_str = caps.get(1).unwrap().as_str();

        let int = int_str.parse::<u64>().map_err(E::custom)?;

        match caps.get(2).unwrap().as_str() {
            "s" => Ok(ParsedDuration(tokio::time::Duration::from_secs(int))),
            "ms" => Ok(ParsedDuration(tokio::time::Duration::from_millis(int))),
            _ => Err(E::custom("invalid duration suffix")),
        }
    }
}

impl<'de> serde::Deserialize<'de> for ParsedDuration {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(ParsedDurationVisitor)
    }
}

impl serde::Serialize for ParsedDuration {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let as_str = format!("{}ms", self.0.as_millis());

        serializer.serialize_str(&as_str)
    }
}

pub mod llm {
    use serde_derive::{Deserialize, Serialize};

    #[derive(Clone, Deserialize, Serialize, Copy, PartialEq, Eq, Debug)]
    pub enum OutputFormat {
        #[serde(rename = "text")]
        Text,
        #[serde(rename = "json")]
        JSON,
    }

    #[derive(Serialize, Deserialize)]
    pub struct PromptIDVarsComparative {
        leader_answer: String,
        validator_answer: String,
        principle: String,
    }

    #[derive(Serialize, Deserialize)]
    pub struct PromptIDVarsNonComparativeValidator {
        pub task: String,
        pub criteria: String,
        pub input: String,
        pub output: String,
    }

    #[derive(Serialize, Deserialize)]
    pub struct PromptIDVarsNonComparativeLeader {
        pub task: String,
        pub criteria: String,
        pub input: String,
    }

    fn default_text() -> OutputFormat {
        OutputFormat::Text
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct Image(#[serde(with = "serde_bytes")] pub Vec<u8>);

    #[derive(Serialize, Deserialize, Debug)]
    pub struct PromptPayload {
        #[serde(default = "default_text")]
        pub response_format: OutputFormat,
        pub prompt: String,
        pub images: Vec<Image>,
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

    #[derive(Serialize, Deserialize)]
    #[serde(tag = "template")]
    pub enum PromptTemplatePayload {
        EqComparative(PromptEqComparativePayload),
        EqNonComparativeValidator(PromptEqNonComparativeValidatorPayload),
        EqNonComparativeLeader(PromptEqNonComparativeLeaderPayload),
    }

    #[derive(Serialize, Deserialize)]
    pub enum Message {
        Prompt(PromptPayload),
        PromptTemplate(PromptTemplatePayload),
    }

    #[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
    #[serde(untagged)]
    pub enum PromptAnswer {
        Text(String),
        Bool(bool),
        Object(serde_json::Map<String, serde_json::Value>),
    }

    impl PromptAnswer {
        pub fn map_text(&mut self, f: impl FnOnce(&mut String)) {
            if let PromptAnswer::Text(t) = self {
                f(t)
            }
        }
    }
}

pub mod web {
    use std::collections::BTreeMap;

    use serde_derive::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    pub enum RenderMode {
        #[serde(rename = "text")]
        Text,
        #[serde(rename = "html")]
        HTML,
        #[serde(rename = "screenshot")]
        Screenshot,
    }

    fn no_wait() -> super::ParsedDuration {
        super::ParsedDuration(tokio::time::Duration::ZERO)
    }

    #[derive(Serialize, Deserialize)]
    pub struct RenderPayload {
        pub mode: RenderMode,
        pub url: String,
        #[serde(default = "no_wait")]
        pub wait_after_loaded: super::ParsedDuration,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub enum RequestMethod {
        GET,
        POST,
        HEAD,
        DELETE,
        OPTIONS,
        PATCH,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct Response {
        pub status: u16,
        pub headers: BTreeMap<String, HeaderData>,

        #[serde(with = "serde_bytes")]
        pub body: Vec<u8>,
    }

    fn default_none<T>() -> Option<T> {
        None
    }

    fn default_false() -> bool {
        false
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct HeaderData(#[serde(with = "serde_bytes")] pub Vec<u8>);

    impl From<HeaderData> for super::GenericValue {
        fn from(val: HeaderData) -> Self {
            val.0.into()
        }
    }

    #[derive(Serialize, Deserialize)]
    pub struct RequestPayload {
        pub method: RequestMethod,
        pub url: String,
        pub headers: BTreeMap<String, HeaderData>,

        #[serde(with = "serde_bytes", default = "default_none")]
        pub body: Option<Vec<u8>>,
        #[serde(default = "default_false")]
        pub sign: bool,
    }

    #[derive(Serialize, Deserialize)]
    pub enum Message {
        Render(RenderPayload),
        Request(RequestPayload),
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

#[derive(Debug, Serialize, Deserialize)]
pub struct GenVMHello {
    pub cookie: String,
    pub host_data: HostData,
}
