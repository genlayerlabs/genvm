use crate::config;

use anyhow::Context;
use genvm_modules_impl_common::*;
use genvm_modules_interfaces::web as web_iface;
use std::sync::Arc;

struct Handler {
    config: Arc<config::Config>,
    client: reqwest::Client,
    session_id: String,
    hello: genvm_modules_interfaces::GenVMHello,
}

impl genvm_modules_impl_common::MessageHandler<web_iface::Message, web_iface::RenderAnswer>
    for Handler
{
    fn handle(
        &self,
        message: web_iface::Message,
    ) -> impl std::future::Future<
        Output = genvm_modules_impl_common::ModuleResult<web_iface::RenderAnswer>,
    > + Send {
        match message {
            web_iface::Message::Render(payload) => self.handle_render(payload),
        }
    }

    async fn cleanup(&self) -> anyhow::Result<()> {
        if let Err(err) = self
            .client
            .delete(format!(
                "{}/session/{}",
                self.config.webdriver_host, self.session_id
            ))
            .send()
            .await
        {
            log::error!(error:err = err, id = self.session_id, cookie = self.hello.cookie; "session closed");
        } else {
            log::debug!(id = self.session_id, cookie = self.hello.cookie; "session closed");
        }
        Ok(())
    }
}

pub struct HandlerProvider {
    pub config: Arc<config::Config>,
}

impl
    genvm_modules_impl_common::MessageHandlerProvider<
        genvm_modules_interfaces::web::Message,
        genvm_modules_interfaces::web::RenderAnswer,
    > for HandlerProvider
{
    async fn new_handler(
        &self,
        hello: genvm_modules_interfaces::GenVMHello,
    ) -> anyhow::Result<
        impl MessageHandler<
            genvm_modules_interfaces::web::Message,
            genvm_modules_interfaces::web::RenderAnswer,
        >,
    > {
        let client = reqwest::Client::new();
        let create_request = client
            .post(format!("{}/session", &self.config.webdriver_host))
            .header("Content-Type", "application/json; charset=utf-8")
            .body(self.config.session_create_request.clone());
        log::trace!(request:? = create_request, body = self.config.session_create_request, cookie = hello.cookie; "creating session");
        let opened_session_res = create_request
            .send()
            .await
            .with_context(|| "creating sessions request")?;
        let body = read_response(opened_session_res)
            .await
            .with_context(|| "reading response")?;
        let val: serde_json::Value = serde_json::from_str(&body)?;
        let session_id = val
            .pointer("/value/sessionId")
            .and_then(|val| val.as_str())
            .ok_or_else(|| anyhow::anyhow!("invalid json {}", val))?;

        Ok(Handler {
            config: self.config.clone(),
            client,
            session_id: session_id.to_owned(),
            hello,
        })
    }
}

impl Handler {
    async fn handle_render(
        &self,
        payload: web_iface::RenderPayload,
    ) -> genvm_modules_impl_common::ModuleResult<web_iface::RenderAnswer> {
        let url = match url::Url::parse(&payload.url) {
            Ok(url) => url,
            Err(_) => {
                return Err(ModuleResultUserError(
                    serde_json::json!({"message": "invalid url", "url": payload.url}),
                )
                .into());
            }
        };
        if url.scheme() == "file" {
            return Err(ModuleResultUserError(
                serde_json::json!({"message": "scheme forbidden", "scheme": "file"}),
            )
            .into());
        }

        match url.host_str() {
            None => {
                return Err(ModuleResultUserError(
                    serde_json::json!({"message": "host is forbidden", "host": null}),
                )
                .into())
            }
            Some(host_str)
                if crate::config::binary_search_contains(
                    &self.config.always_allow_hosts,
                    host_str,
                ) => {}
            Some(host_str) => {
                if !self.config.tld_is_ok(host_str) {
                    return Err(ModuleResultUserError(
                        serde_json::json!({"message": "tld forbidden", "host": host_str}),
                    )
                    .into());
                }

                const ALLOWED_PORTS: &[Option<u16>] = &[None, Some(80), Some(443)];
                if !ALLOWED_PORTS.contains(&url.port()) {
                    return Err(ModuleResultUserError(
                        serde_json::json!({"message": "port forbidden", "port": url.port()}),
                    )
                    .into());
                }
            }
        }

        let req_body = serde_json::json!({
            "url": url.as_str()
        });
        let req_body = serde_json::to_string(&req_body)?;
        let req = self
            .client
            .post(format!(
                "{}/session/{}/url",
                self.config.webdriver_host, self.session_id
            ))
            .header("Content-Type", "application/json; charset=utf-8")
            .body(req_body.clone());

        log::info!(request:? = req, body = req_body, cookie = self.hello.cookie; "sending request");

        let res = req.send().await?;
        let res = res.error_for_status()?;
        std::mem::drop(res);

        match payload.wait_after_loaded {
            genvm_modules_interfaces::ParsedDuration(tokio::time::Duration::ZERO) => {}
            genvm_modules_interfaces::ParsedDuration(x) => {
                log::trace!(duration:? = x, cookie = self.hello.cookie; "sleeping to allow page to load");
                tokio::time::sleep(x).await
            }
        }

        let script = match payload.mode {
            web_iface::RenderMode::HTML => {
                r#"{ "script": "return document.body.innerHTML", "args": [] }"#
            }
            web_iface::RenderMode::Text => {
                r#"{ "script": "return document.body.innerText.replace(/[\\s\\n]+/g, ' ')", "args": [] }"#
            }
        };

        let req = self
            .client
            .post(format!(
                "{}/session/{}/execute/sync",
                self.config.webdriver_host, self.session_id
            ))
            .header("Content-Type", "application/json; charset=utf-8")
            .body(script);
        log::debug!(request:? = req, body = script, cookie = self.hello.cookie; "getting web page data");

        let res = req.send().await?;

        let res = res.error_for_status()?;

        let body = res.text().await?;

        let res_buf = body;

        let val: serde_json::Value = serde_json::from_str(&res_buf)?;
        let val = val
            .pointer("/value")
            .and_then(|x| x.as_str())
            .ok_or_else(|| anyhow::anyhow!("invalid json {}", val))?;

        Ok(genvm_modules_interfaces::web::RenderAnswer::Text(
            String::from(val.trim()),
        ))
    }
}
