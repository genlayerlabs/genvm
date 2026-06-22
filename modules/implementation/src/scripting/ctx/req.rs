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

/// NOTE: when changing fields, also update request in modules/install/lib/genvm-lua/lib-genvm.lua
#[derive(Debug, Serialize, Deserialize)]
pub struct Request {
    pub method: web_iface::RequestMethod,
    pub url: url::Url,
    pub headers: BTreeMap<String, bytes::Bytes>,

    #[serde(default)]
    pub response_body_max_size: Option<usize>,

    #[serde(with = "serde_bytes", default = "default_none")]
    pub body: Option<Vec<u8>>,
    #[serde(default = "default_false")]
    pub sign: bool,
    #[serde(default = "default_false")]
    pub json: bool,
    #[serde(default = "default_false")]
    pub error_on_status: bool,
    #[serde(default)]
    pub timeout: Option<crate::common::Timeout>,

    /// Send through the plain (non-filtering) client instead of the SSRF-guarded
    /// one. Set by the web module for hosts in `always_allow_hosts`.
    #[serde(default = "default_false")]
    pub unfiltered: bool,

    /// Set once `normalize_headers` has run. Lets `into_reqwest` normalize every
    /// request exactly once without re-stripping the `genlayer-*` headers the
    /// signing path legitimately adds after normalizing. Not part of the wire
    /// format.
    #[serde(skip)]
    pub headers_normalized: bool,
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

        self.headers_normalized = true;
    }

    pub fn into_reqwest(
        mut self,
        client: &reqwest::Client,
    ) -> Result<reqwest::RequestBuilder, ModuleError> {
        // Drop forbidden/forged headers (host, content-length, genlayer-*, @*)
        // on every request. The signing path normalizes earlier (before adding
        // the legitimate genlayer-* headers), so the flag keeps us from
        // stripping those back out here.
        if !self.headers_normalized {
            self.normalize_headers();
        }

        let method = match self.method {
            web_iface::RequestMethod::GET => reqwest::Method::GET,
            web_iface::RequestMethod::POST => reqwest::Method::POST,
            web_iface::RequestMethod::DELETE => reqwest::Method::DELETE,
            web_iface::RequestMethod::PUT => reqwest::Method::PUT,
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
            let data: &[u8] = v.as_ref();
            headers.insert(
                name,
                data.try_into()
                    .map_user_error_module("invalid header value", true)?,
            );
        }

        let request = client.request(method, self.url.clone()).headers(headers);

        let request = if let Some(body) = self.body {
            request.body(body)
        } else {
            request
        };
        Ok(if let Some(timeout) = self.timeout {
            request.timeout(timeout.to_duration())
        } else {
            request
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common;
    use genvm_common::*;
    use genvm_modules_interfaces::web as web_iface;

    #[tokio::test]
    async fn test_response_body_max_size_ok() {
        common::tests::setup();

        let client = common::tests::create_test_client();
        let metrics = sync::DArc::new(crate::scripting::ctx::Metrics::default());
        let url = "https://test-server.genlayer.com/body/echo";

        // body within limit (500 bytes < 1024 limit) => success
        let body = vec![b'a'; 500];
        let req = crate::scripting::ctx::req::Request {
            url: url::Url::parse(url).unwrap(),
            method: web_iface::RequestMethod::POST,
            headers: BTreeMap::new(),
            body: Some(body.clone()),
            json: false,
            error_on_status: false,
            sign: false,
            response_body_max_size: Some(1024),
            timeout: None,
            unfiltered: false,
            headers_normalized: false,
        };

        let body_size_limit = req.response_body_max_size.unwrap_or(usize::MAX);
        let reqwst = req.into_reqwest(&client).unwrap();

        let res = crate::scripting::send_request_get_lua_compatible_response_bytes(
            &metrics,
            url,
            reqwst,
            false,
            body_size_limit,
        )
        .await
        .unwrap();

        assert_eq!(res.status, 200);
        assert_eq!(res.body, body);
    }

    #[tokio::test]
    async fn test_response_body_max_size_exceeded() {
        common::tests::setup();

        let client = common::tests::create_test_client();
        let metrics = sync::DArc::new(crate::scripting::ctx::Metrics::default());
        let url = "https://test-server.genlayer.com/body/echo";

        let body = vec![b'b'; 2000];
        let req = crate::scripting::ctx::req::Request {
            url: url::Url::parse(url).unwrap(),
            method: web_iface::RequestMethod::POST,
            headers: BTreeMap::new(),
            body: Some(body),
            json: false,
            error_on_status: false,
            sign: false,
            response_body_max_size: Some(1024),
            timeout: None,
            unfiltered: false,
            headers_normalized: false,
        };

        let body_size_limit = 1024;
        let reqwst = req.into_reqwest(&client).unwrap();

        let err = crate::scripting::send_request_get_lua_compatible_response_bytes(
            &metrics,
            url,
            reqwst,
            false,
            body_size_limit,
        )
        .await
        .unwrap_err();

        log_info!(error:ah = &err; "Received expected error for exceeding body size limit");

        let module_err = err.downcast::<crate::common::ModuleError>().unwrap();
        assert!(module_err.causes.contains(&"READING_BODY".to_owned()));
    }
}
