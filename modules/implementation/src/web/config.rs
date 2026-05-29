use serde::Serialize;
use serde_derive::Deserialize;

use crate::common;

fn default_json_null() -> serde_json::Value {
    serde_json::Value::Null
}

/// NOTE: when changing fields, also update doc/schemas/default-config.json
#[derive(Serialize, Deserialize)]
pub struct Config {
    pub webdriver_host: String,

    pub extra_tld: Vec<Box<str>>,
    pub always_allow_hosts: Vec<Box<str>>,

    #[serde(default = "default_json_null")]
    pub meta: serde_json::Value,

    #[serde(flatten)]
    pub base: genvm_common::BaseConfig,

    #[serde(flatten)]
    pub mod_base: common::ModuleBaseConfig,
}
