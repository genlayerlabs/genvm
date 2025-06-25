use std::collections::BTreeMap;

use serde_derive::{Deserialize, Serialize};

use crate::common;

use super::providers;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, clap::ValueEnum)]
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

fn default_false() -> bool {
    false
}

fn default_json_null() -> serde_json::Value {
    serde_json::Value::Null
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ModelConfig {
    #[serde(default = "enabled_true")]
    pub enabled: bool,
    #[serde(default = "default_false")]
    pub supports_json: bool,
    #[serde(default = "default_false")]
    pub supports_image: bool,
    #[serde(default = "default_false")]
    pub use_max_completion_tokens: bool,

    #[serde(default = "default_json_null")]
    pub meta: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ScriptBackendConfig {
    pub models: BTreeMap<String, ModelConfig>,
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

#[derive(Serialize, Deserialize)]
pub struct PromptTemplates {
    pub eq_comparative: serde_json::Value,
    pub eq_non_comparative_leader: serde_json::Value,
    pub eq_non_comparative_validator: serde_json::Value,
}

#[derive(Deserialize)]
pub struct Config {
    pub backends: BTreeMap<String, BackendConfig>,
    pub prompt_templates: PromptTemplates,

    #[serde(flatten)]
    pub base: genvm_common::BaseConfig,

    #[serde(flatten)]
    pub mod_base: common::ModuleBaseConfig,
}

impl BackendConfig {
    pub fn to_provider(&self) -> Box<dyn providers::Provider + Send + Sync> {
        match self.provider {
            Provider::Ollama => Box::new(providers::OLlama {
                config: self.clone(),
            }),
            Provider::OpenaiCompatible => Box::new(providers::OpenAICompatible {
                config: self.clone(),
            }),
            Provider::Anthropic => Box::new(providers::Anthropic {
                config: self.clone(),
            }),
            Provider::Google => Box::new(providers::Gemini {
                config: self.clone(),
            }),
        }
    }
}
