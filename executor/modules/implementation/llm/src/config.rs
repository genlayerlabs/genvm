use std::collections::BTreeMap;

use serde_derive::{Deserialize, Serialize};

use crate::providers;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Provider {
    Ollama,
    OpenaiCompatible,
    Anthropic,
    Google,
}

fn enabled_true() -> bool {
    true
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ScriptBackendConfig {
    pub models: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BackendConfig {
    #[serde(default = "enabled_true")]
    pub enabled: bool,
    pub host: String,
    pub provider: Provider,
    pub key: String,

    #[serde(flatten)]
    pub script_config: ScriptBackendConfig,
}

#[derive(Deserialize)]
pub struct PromptTemplates {
    pub eq_comparative: String,
    pub eq_non_comparative_leader: String,
    pub eq_non_comparative_validator: String,
}

#[derive(Deserialize)]
pub struct Config {
    pub bind_address: String,

    pub lua_script_path: String,

    pub backends: BTreeMap<String, BackendConfig>,
    pub prompt_templates: PromptTemplates,

    #[serde(flatten)]
    pub base: genvm_common::BaseConfig,
}

impl BackendConfig {
    pub fn to_provider(
        &self,
        client: reqwest::Client,
    ) -> Box<dyn providers::Provider + Send + Sync> {
        match self.provider {
            Provider::Ollama => Box::new(providers::OLlama {
                client,
                config: self.clone(),
            }),
            Provider::OpenaiCompatible => Box::new(providers::OpenAICompatible {
                client,
                config: self.clone(),
            }),
            Provider::Anthropic => Box::new(providers::Anthropic {
                client,
                config: self.clone(),
            }),
            Provider::Google => Box::new(providers::Gemini {
                client,
                config: self.clone(),
            }),
        }
    }
}
