use serde::Serialize;
use serde_derive::Deserialize;

use crate::common;

fn default_json_null() -> serde_json::Value {
    serde_json::Value::Null
}

fn default_max_wait_after_loaded() -> common::Timeout {
    common::Timeout::from_secs(60)
}

/// NOTE: when changing fields, also update doc/schemas/default-config.json
#[derive(Serialize, Deserialize)]
pub struct Config {
    pub webdriver_host: String,

    pub extra_tld: Vec<Box<str>>,
    pub always_allow_hosts: Vec<Box<str>>,

    #[serde(default = "default_json_null")]
    pub meta: serde_json::Value,

    /// Upper bound for `wait_after_loaded` requested by contracts. Values above
    /// this are clamped (with a warning) to prevent unbounded execution stalls.
    #[serde(default = "default_max_wait_after_loaded")]
    pub max_wait_after_loaded: common::Timeout,

    #[serde(flatten)]
    pub base: genvm_common::BaseConfig,

    #[serde(flatten)]
    pub mod_base: common::ModuleBaseConfig,
}
