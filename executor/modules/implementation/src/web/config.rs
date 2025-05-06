use serde_derive::Deserialize;
use std::borrow::Borrow;

use super::domains;

#[derive(Deserialize)]
pub struct Config {
    pub bind_address: String,
    pub webdriver_host: String,
    pub session_create_request: String,

    pub extra_tld: Vec<Box<str>>,
    pub always_allow_hosts: Vec<Box<str>>,

    #[serde(flatten)]
    pub base: genvm_common::BaseConfig,
}

pub fn binary_search_contains<T, Y>(arr: &[T], val: Y) -> bool
where
    T: Borrow<str>,
    Y: Borrow<str>,
{
    arr.binary_search_by(|x| {
        let x: &str = x.borrow();
        x.cmp(val.borrow())
    })
    .is_ok()
}

impl Config {
    pub fn tld_is_ok(&self, host: &str) -> bool {
        let tld = match host.rfind(".") {
            None => host,
            Some(idx) => &host[idx + 1..],
        };

        if binary_search_contains(domains::DOMAINS, tld) {
            return true;
        }

        if binary_search_contains(&self.extra_tld, tld) {
            return true;
        }

        false
    }
}
