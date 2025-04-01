use std::collections::BTreeMap;

use serde_derive::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
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

fn no_key() -> String {
    "".to_owned()
}

#[derive(Serialize, Deserialize)]
pub struct BackendConfig {
    #[serde(default = "enabled_true")]
    pub enabled: bool,
    pub host: String,
    pub provider: Provider,
    pub model: String,
    pub key_env_name: String,
    #[serde(default = "no_key")]
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

    pub backends: BTreeMap<String, BackendConfig>,
    pub prompt_templates: PromptTemplates,

    pub threads: usize,
    pub blocking_threads: usize,
}
