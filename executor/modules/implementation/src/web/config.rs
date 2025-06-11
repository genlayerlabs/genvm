use serde::Serialize;
use serde_derive::Deserialize;

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub bind_address: String,
    pub webdriver_host: String,
    pub session_create_request: String,

    pub lua_script_path: String,
    pub vm_count: usize,

    pub extra_tld: Vec<Box<str>>,
    pub always_allow_hosts: Vec<Box<str>>,

    #[serde(flatten)]
    pub base: genvm_common::BaseConfig,
}
