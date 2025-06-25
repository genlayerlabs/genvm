use std::collections::BTreeMap;

use super::req::Request;
use crate::common::{ErrorKind, ModuleError};
use base64::Engine;
use genvm_modules_interfaces::web as web_iface;
use genvm_modules_interfaces::GenericValue;

const SIGN_ALGORITHM: &str = "ES256K";

const ALWAYS_SIGN: &[&str] = &[
    "@method",
    "@authority",
    "@path",
    "genlayer-node-address",
    "genlayer-tx-id",
    "genlayer-salt",
];

fn map_fmt_to_fatal(e: std::fmt::Error) -> ModuleError {
    ModuleError {
        causes: vec!["WRITE_FORMAT_FAILED".to_owned()],
        ctx: BTreeMap::from([("rust_error".to_owned(), format!("{e:#}").into())]),
        fatal: true,
    }
}

impl Request {
    pub async fn add_rfc9421_sign_headers(
        &mut self,
        ctx: &super::dflt::CtxPart,
    ) -> Result<(), ModuleError> {
        self.normalize_headers();
        self.add_node_headers(
            &mut ring::rand::SystemRandom::new(),
            &ctx.node_address,
            &ctx.tx_id,
        )?;

        self.add_content_digest_header()?;

        let components = self.sign_get_components()?;

        let (signature_params, signature_base) = self.sign_get_signature(&components)?;

        let sign_url = genvm_common::templater::patch_str(
            &ctx.sign_vars,
            &ctx.sign_url,
            &genvm_common::templater::HASH_UNFOLDER_RE,
        )
        .map_err(|e| ModuleError {
            causes: vec!["SIGN_URL_PATCH_FAILED".to_owned()],
            ctx: BTreeMap::from([("error".to_string(), GenericValue::Str(e.to_string()))]),
            fatal: true,
        })?;

        let mut sign_request = ctx.client.post(sign_url).body(signature_base.clone());

        //let signature_base_hashed = ring::digest::digest(&ring::digest::SHA256, signature_base.as_bytes());
        //let signature_base_hashed = Vec::from(signature_base_hashed.as_ref());
        //let mut sign_request = ctx.client.post(sign_url).body(signature_base_hashed);

        for (k, v) in ctx.sign_headers.iter() {
            let new_v = genvm_common::templater::patch_str(
                &ctx.sign_vars,
                v,
                &genvm_common::templater::HASH_UNFOLDER_RE,
            )
            .map_err(|e| ModuleError {
                causes: vec!["SIGN_URL_PATCH_FAILED".to_owned()],
                ctx: BTreeMap::from([("error".to_string(), GenericValue::Str(e.to_string()))]),
                fatal: true,
            })?;

            sign_request = sign_request.header(k, new_v);
        }

        let resp = sign_request.send().await.map_err(|e| ModuleError {
            causes: vec!["SIGN_URL_SEND_FAILED".to_owned()],
            ctx: BTreeMap::from([("error".to_string(), GenericValue::Str(e.to_string()))]),
            fatal: true,
        })?;

        if resp.status() != 200 {
            return Err(ModuleError {
                causes: vec!["SIGN_URL_BAD_STATUS".to_owned()],
                ctx: BTreeMap::from([(
                    "status".to_string(),
                    GenericValue::Str(resp.status().to_string()),
                )]),
                fatal: true,
            });
        }

        let signature = resp.bytes().await.map_err(|e| ModuleError {
            causes: vec!["SIGN_URL_READ_FAILED".to_owned()],
            ctx: BTreeMap::from([("error".to_string(), GenericValue::Str(e.to_string()))]),
            fatal: true,
        })?;

        self.sign_add_signature_headers(signature.as_ref(), &signature_params)?;

        Ok(())
    }

    fn rfc9421_get_component_value(&self, component: &str) -> Result<String, ModuleError> {
        match component {
            "@method" => Ok(format!("{:?}", self.method)),
            "@authority" => Ok(self.url.authority().to_lowercase()),
            "@path" => Ok(self.url.path().to_owned()),
            "@query" => self
                .url
                .query()
                .map(|x| {
                    let mut query = String::from("?");
                    query.push_str(x);
                    query
                })
                .ok_or_else(|| ModuleError {
                    causes: vec!["QUERY_NOT_PRESENT".to_owned()],
                    ctx: BTreeMap::from([(
                        "url".to_string(),
                        GenericValue::Str(self.url.to_string()),
                    )]),
                    fatal: true,
                }),
            "@target-uri" => Ok(self.url.to_string()),
            component => {
                // Regular header
                self.headers
                    .get(component)
                    .map(|h| String::from_utf8_lossy(&h.0).to_string())
                    .ok_or_else(|| ModuleError {
                        causes: vec![ErrorKind::ABSENT_HEADER.into()],
                        ctx: BTreeMap::from([
                            (
                                "header".to_string(),
                                GenericValue::Str(component.to_string()),
                            ),
                            ("url".to_string(), GenericValue::Str(self.url.to_string())),
                        ]),
                        fatal: false,
                    })
            }
        }
    }

    pub fn add_node_headers(
        &mut self,
        rand: &mut impl ring::rand::SecureRandom,
        node_address: &str,
        tx_id: &str,
    ) -> Result<(), ModuleError> {
        self.headers.insert(
            "genlayer-node-address".to_owned(),
            web_iface::HeaderData(node_address.into()),
        );

        self.headers.insert(
            "genlayer-tx-id".to_owned(),
            web_iface::HeaderData(tx_id.into()),
        );

        let mut salt = [0; 32];
        rand.fill(&mut salt).map_err(|e| ModuleError {
            causes: vec!["RANDOM_GENERATION".to_owned()],
            ctx: BTreeMap::from([("error".to_string(), GenericValue::Str(e.to_string()))]),
            fatal: true,
        })?;

        self.headers.insert(
            "genlayer-salt".to_owned(),
            web_iface::HeaderData(base64::prelude::BASE64_STANDARD.encode(salt).into()),
        );

        Ok(())
    }

    pub fn add_content_digest_header(&mut self) -> Result<(), ModuleError> {
        if let Some(body) = &self.body {
            let digest = ring::digest::digest(&ring::digest::SHA256, body);

            let digest_value = format!(
                "sha-256=:{}:",
                base64::prelude::BASE64_STANDARD.encode(digest)
            );

            self.headers.insert(
                "content-digest".to_string(),
                web_iface::HeaderData(digest_value.into_bytes()),
            );
        }

        Ok(())
    }

    pub(self) fn sign_get_components(&self) -> Result<Vec<String>, ModuleError> {
        let mut components = Vec::new();

        components.extend(ALWAYS_SIGN.iter().map(|s| (*s).to_owned()));

        if self.headers.contains_key("content-digest") {
            components.push("content-digest".to_string());
        }

        if self.url.query().is_some() {
            components.push("@query".to_string());
        }

        Ok(components)
    }

    pub(self) fn sign_get_signature<S>(
        &self,
        components: &[S],
    ) -> Result<(String, String), ModuleError>
    where
        S: AsRef<str>,
    {
        use std::fmt::Write as _;

        let mut signature_params = String::new();
        let mut signature_value = String::new();

        signature_params.push('(');

        let mut first = true;

        for c in components {
            let c = c.as_ref();

            if first {
                first = false;
            } else {
                signature_params.push(' ');
            }

            signature_params.push('"');
            signature_params.push_str(c);
            signature_params.push('"');

            let value = self.rfc9421_get_component_value(c)?;
            writeln!(signature_value, "\"{}\": {}", c, value).map_err(map_fmt_to_fatal)?;
        }

        let created = chrono::Utc::now().timestamp();
        write!(
            signature_params,
            ");created={};alg=\"{}\"",
            created, SIGN_ALGORITHM
        )
        .map_err(map_fmt_to_fatal)?;

        write!(
            signature_value,
            "\"@signature-params\": {}",
            signature_params
        )
        .map_err(map_fmt_to_fatal)?;

        Ok((signature_params, signature_value))
    }

    fn sign_add_signature_headers(
        &mut self,
        signature: &[u8],
        params: &str,
    ) -> Result<(), ModuleError> {
        // Add Signature-Input header
        let sig_input = format!("genvm={}", params);
        self.headers.insert(
            "signature-input".to_string(),
            web_iface::HeaderData(sig_input.into_bytes()),
        );

        // Add Signature header
        let sig_value = format!(
            "genvm=:{}:",
            base64::prelude::BASE64_STANDARD.encode(signature)
        );
        self.headers.insert(
            "signature".to_string(),
            web_iface::HeaderData(sig_value.into_bytes()),
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::common;
    use genvm_modules_interfaces::web as web_iface;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_signing_get() {
        common::tests::setup();

        let mut req = crate::scripting::ctx::req::Request {
            url: url::Url::parse("https://example.com/foo?a=b").unwrap(),
            method: web_iface::RequestMethod::GET,
            headers: BTreeMap::new(),
            body: None,
            json: false,
            error_on_status: true,
            sign: true,
        };

        req.normalize_headers();
        req.add_node_headers(
            &mut ring::rand::SystemRandom::new(),
            "test_address",
            "test_tx_id",
        )
        .unwrap();
        req.add_content_digest_header().unwrap();

        let components = req.sign_get_components().unwrap();

        let (signature_params, signature_base) = req.sign_get_signature(&components).unwrap();

        let signature_params = regex::Regex::new(r#"created=\d+"#)
            .unwrap()
            .replace(&signature_params, "created=1750171014");

        let signature_base = regex::Regex::new(r#"created=\d+"#)
            .unwrap()
            .replace(&signature_base, "created=1750171014");
        let signature_base = regex::Regex::new(r#""genlayer-salt": [^\n]+"#)
            .unwrap()
            .replace(&signature_base, "\"genlayer-salt\": <replaced>");

        assert_eq!(
            signature_params,
            r#"("@method" "@authority" "@path" "genlayer-node-address" "genlayer-tx-id" "genlayer-salt" "@query");created=1750171014;alg="ES256K""#
        );
        let base = r#"
"@method": GET
"@authority": example.com
"@path": /foo
"genlayer-node-address": test_address
"genlayer-tx-id": test_tx_id
"genlayer-salt": <replaced>
"@query": ?a=b
"@signature-params": ("@method" "@authority" "@path" "genlayer-node-address" "genlayer-tx-id" "genlayer-salt" "@query");created=1750171014;alg="ES256K"
        "#;
        let base = base.trim();
        assert_eq!(signature_base.trim(), base);
    }

    #[tokio::test]
    async fn test_signing_post() {
        common::tests::setup();

        let mut req = crate::scripting::ctx::req::Request {
            url: url::Url::parse("https://example.com/pst").unwrap(),
            method: web_iface::RequestMethod::POST,
            headers: BTreeMap::new(),
            body: Some(b"test body".to_vec()),
            json: false,
            error_on_status: true,
            sign: true,
        };

        req.normalize_headers();
        req.add_node_headers(
            &mut ring::rand::SystemRandom::new(),
            "test_address",
            "test_tx_id",
        )
        .unwrap();
        req.add_content_digest_header().unwrap();

        let components = req.sign_get_components().unwrap();

        let (signature_params, signature_base) = req.sign_get_signature(&components).unwrap();

        let signature_params = regex::Regex::new(r#"created=\d+"#)
            .unwrap()
            .replace(&signature_params, "created=1750171014");

        let signature_base = regex::Regex::new(r#"created=\d+"#)
            .unwrap()
            .replace(&signature_base, "created=1750171014");
        let signature_base = regex::Regex::new(r#""genlayer-salt": [^\n]+"#)
            .unwrap()
            .replace(&signature_base, "\"genlayer-salt\": <replaced>");

        assert_eq!(
            signature_params,
            r#"("@method" "@authority" "@path" "genlayer-node-address" "genlayer-tx-id" "genlayer-salt" "content-digest");created=1750171014;alg="ES256K""#
        );
        let base = r#"
"@method": POST
"@authority": example.com
"@path": /pst
"genlayer-node-address": test_address
"genlayer-tx-id": test_tx_id
"genlayer-salt": <replaced>
"content-digest": sha-256=:Y++zFe1xzH5aH8ICQ0uzrsIJHng4cH4UigF/rrt0ZP4=:
"@signature-params": ("@method" "@authority" "@path" "genlayer-node-address" "genlayer-tx-id" "genlayer-salt" "content-digest");created=1750171014;alg="ES256K"
        "#;
        let base = base.trim();
        assert_eq!(signature_base.trim(), base);
    }

    #[tokio::test]
    async fn test_signing_post_with_server() {
        use crate::scripting::ctx::dflt::CtxPart;

        common::tests::setup();

        let mut req = crate::scripting::ctx::req::Request {
            url: url::Url::parse("http://test-server.genlayer.com/body/echo-signed").unwrap(),
            method: web_iface::RequestMethod::POST,
            headers: BTreeMap::new(),
            body: Some(b"test body".to_vec()),
            json: false,
            error_on_status: true,
            sign: true,
        };

        let part = CtxPart {
            client: reqwest::Client::new(),
            sign_url: Arc::from("https://test-server.genlayer.com/genvm/sign"),
            sign_headers: Arc::new(BTreeMap::new()),
            sign_vars: BTreeMap::new(),
            node_address: "node_address".to_string(),
            tx_id: "tx_id".to_string(),
        };

        req.add_rfc9421_sign_headers(&part).await.unwrap();

        eprintln!("req: {req:?}");

        let reqwst = req.into_reqwest(&part.client).unwrap();

        let res = reqwst.send().await.unwrap();
        let status = res.status();
        let body = res.bytes().await;
        eprintln!("Response status: {:?}, body: {:?}", status, body);
        assert_eq!(status, 200);

        let body = body.unwrap();

        assert_eq!(body.as_ref(), b"test body");
    }
}
