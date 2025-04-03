use std::collections::BTreeMap;

use serde_derive::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Provider {
    Ollama,
    OpenaiCompatible,
    Simulator,
    Anthropic,
    Google,
}

fn enabled_true() -> bool {
    true
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BackendConfig {
    #[serde(default = "enabled_true")]
    pub enabled: bool,
    pub host: String,
    pub provider: Provider,
    pub model: String,
    pub key: String,
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
