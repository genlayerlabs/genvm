use std::collections::BTreeMap;

use genvm_modules_interfaces::web as web_iface;
use serde::{Deserialize, Serialize};

fn default_none<T>() -> Option<T> {
    None
}

fn default_false() -> bool {
    false
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Request {
    pub method: web_iface::RequestMethod,
    pub url: url::Url,
    pub headers: BTreeMap<String, web_iface::HeaderData>,

    #[serde(with = "serde_bytes", default = "default_none")]
    pub body: Option<Vec<u8>>,
    #[serde(default = "default_false")]
    pub sign: bool,
    #[serde(default = "default_false")]
    pub json: bool,
    #[serde(default = "default_false")]
    pub error_on_status: bool,
}

const DROP_HEADERS: &[&str] = &[
    "content-length",
    "host",
    "genlayer-node-address",
    "genlayer-tx-id",
    "genlayer-salt",
];

impl Request {
    pub fn normalize_headers(&mut self) {
        let mut old_headers = BTreeMap::new();
        std::mem::swap(&mut self.headers, &mut old_headers);

        for (k, v) in old_headers.into_iter() {
            let lower_k = k.to_lowercase();

            if DROP_HEADERS.contains(&lower_k.trim()) {
                continue;
            }

            if lower_k.starts_with("@") {
                continue;
            }

            self.headers.insert(lower_k, v);
        }
    }
}
