use serde_derive::Deserialize;

#[derive(Deserialize)]
pub struct Module {
    pub address: String,
}

#[derive(Deserialize)]
pub struct Modules {
    pub llm: Module,
    pub web: Module,
}

#[derive(Deserialize)]
pub struct Config {
    pub modules: Modules,

    #[serde(flatten)]
    pub base: genvm_common::BaseConfig,
}
