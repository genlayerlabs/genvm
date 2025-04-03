use serde_derive::{Deserialize, Serialize};

pub trait Web {
    fn get_webpage(
        &self,
        config: String,
        url: String,
    ) -> tokio::task::JoinHandle<anyhow::Result<Box<[u8]>>>;
}

#[derive(Clone, Deserialize, Serialize)]
pub enum Result<T> {
    Ok(T),
    UserError(serde_json::Value),
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

    #[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
    pub enum PromptPart {
        #[serde(rename = "text")]
        Text(String),
    }

    fn default_text() -> OutputFormat {
        OutputFormat::Text
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct PromptPayload {
        #[serde(default = "default_text")]
        pub response_format: OutputFormat,
        pub parts: Vec<PromptPart>,
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

    #[derive(Serialize, Deserialize, Debug)]
    #[serde(untagged)]
    pub enum PromptAnswer {
        Text(String),
        Bool(bool),
        Object(serde_json::Map<String, serde_json::Value>),
    }
}

pub mod web {
    use serde_derive::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    pub enum RenderMode {
        #[serde(rename = "text")]
        Text,
        #[serde(rename = "html")]
        HTML,
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

    #[derive(Serialize, Deserialize)]
    pub enum Message {
        Render(RenderPayload),
    }

    #[derive(Serialize, Deserialize)]
    pub enum RenderAnswer {
        #[serde(rename = "text")]
        Text(String),
    }
}
