use std::collections::BTreeMap;

use crate::common::ErrorKind;
use crate::common::MapUserError;
use crate::scripting::ModuleError;
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

    pub fn into_reqwest(
        self,
        client: &reqwest::Client,
    ) -> Result<reqwest::RequestBuilder, ModuleError> {
        let method = match self.method {
            web_iface::RequestMethod::GET => reqwest::Method::GET,
            web_iface::RequestMethod::POST => reqwest::Method::POST,
            web_iface::RequestMethod::DELETE => reqwest::Method::DELETE,
            web_iface::RequestMethod::HEAD => reqwest::Method::HEAD,
            web_iface::RequestMethod::OPTIONS => reqwest::Method::OPTIONS,
            web_iface::RequestMethod::PATCH => reqwest::Method::PATCH,
        };

        let mut headers: reqwest::header::HeaderMap<reqwest::header::HeaderValue> =
            reqwest::header::HeaderMap::with_capacity(self.headers.len());
        for (k, v) in self.headers.into_iter() {
            let name: reqwest::header::HeaderName = k
                .as_bytes()
                .try_into()
                .map_user_error_module(ErrorKind::DESERIALIZING.to_string(), true)?;
            let data: &[u8] = &v.0;
            headers.insert(
                name,
                data.try_into()
                    .map_user_error_module("invalid header value", true)?,
            );
        }

        let request = client.request(method, self.url.clone()).headers(headers);

        Ok(if let Some(body) = self.body {
            request.body(body)
        } else {
            request
        })
    }
}
