use serde::Serialize;
use serde_derive::Deserialize;

use crate::common;

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub webdriver_host: String,
    pub session_create_request: String,

    pub extra_tld: Vec<Box<str>>,
    pub always_allow_hosts: Vec<Box<str>>,

    #[serde(flatten)]
    pub base: genvm_common::BaseConfig,

    #[serde(flatten)]
    pub mod_base: common::ModuleBaseConfig,
}
