use anyhow::Result;

use genvm_modules_impl_common::*;
use genvm_modules_interfaces::{CtorArgs, ModuleError, ModuleResult};

use serde_derive::Deserialize;

use std::sync::Arc;

struct Session {
    id: Box<str>,
    host: Arc<str>,
}

impl Drop for Session {
    fn drop(&mut self) {
        let builder = isahc::Request::delete(&format!("{}/session/{}", self.host, self.id));
        let _ = isahc::send(builder.body(()).unwrap());
    }
}

struct Impl {
    sessions: crossbeam::queue::ArrayQueue<Session>,
    config: Config,
    host: Arc<str>,
}

#[derive(Deserialize)]
struct Config {
    host: Arc<str>,
}

#[derive(Deserialize)]
#[allow(non_camel_case_types)]
enum GetWebpageConfigMode {
    html,
    text,
}
#[derive(Deserialize)]
struct GetWebpageConfig {
    mode: GetWebpageConfigMode,
}

unsafe impl Send for Impl {}

impl Impl {
    fn get_session(&self) -> ModuleResult<Session> {
        match self.sessions.pop() {
            Some(s) => return Ok(s),
            None => {}
        }

        const INIT_REQUEST: &str = r#"{
            "capabilities": {
                "alwaysMatch": {
                    "browserName": "chrome",
                    "goog:chromeOptions": {
                        "args": ["--headless", "--disable-dev-shm-usage", "--no-zygote", "--no-sandbox"]
                    }
                }
            }
        }"#;

        let req = isahc::Request::post(&format!("{}/session", &self.config.host))
            .header("Content-Type", "application/json; charset=utf-8")
            .body(INIT_REQUEST)?;
        log::debug!(request:? = req; "creating session");
        let mut opened_session_res = isahc::send(req)?;
        let body = genvm_modules_impl_common::read_response(&mut opened_session_res)?;
        let val: serde_json::Value = serde_json::from_str(&body)?;
        let session_id = val
            .pointer("/value/sessionId")
            .and_then(|val| val.as_str())
            .ok_or(anyhow::anyhow!("invalid json {}", val))?;

        Ok(Session {
            id: Box::from(session_id),
            host: self.host.clone(),
        })
    }
}

impl genvm_modules_interfaces::Web for Impl {
    fn get_webpage(&self, _gas: &mut u64, config: &str, url: &str) -> ModuleResult<String> {
        let config: GetWebpageConfig = serde_json::from_str(config)?;
        let url = url::Url::parse(url)?;
        if url.scheme() == "file" {
            return Err(ModuleError::Recoverable("file scheme is forbidden"));
        }

        if url.host_str() != Some("genvm-test") {
            const ALLOWED_PORTS: &[Option<u16>] = &[None, Some(80), Some(443)];
            if !ALLOWED_PORTS.contains(&url.port()) {
                return Err(ModuleError::Recoverable("port is forbidden"));
            }
        }

        //let should_quit = self.should_quit;
        let res_buf: Option<ModuleResult<String>> = run_with_termination(
            async move {
                let session = self.get_session()?;

                let client = reqwest::Client::new();
                let req_body = serde_json::json!({
                    "url": url.as_str()
                });
                let req_body = serde_json::to_string(&req_body)?;
                let req = client
                    .post(&format!("{}/session/{}/url", self.config.host, session.id))
                    .header("Content-Type", "application/json; charset=utf-8")
                    .body(req_body.clone());

                log::info!(request:? = req, body = req_body; "sending request");

                let res = req.send().await?;
                let res = res.error_for_status()?;
                std::mem::drop(res);

                let script = match config.mode {
                    GetWebpageConfigMode::html => {
                        r#"{ "script": "return document.body.innerHTML", "args": [] }"#
                    }
                    GetWebpageConfigMode::text => {
                        r#"{ "script": "return document.body.innerText.replace(/[\\s\\n]+/g, ' ')", "args": [] }"#
                    }
                };

                let req = client
                    .post(&format!(
                        "{}/session/{}/execute/sync",
                        self.config.host, session.id
                    ))
                    .header("Content-Type", "application/json; charset=utf-8")
                    .body(script);
                log::debug!(request:? = req, body = script; "getting web page data");

                let res = req.send().await?;

                let res = res.error_for_status()?;

                let body = res.text().await?;

                let _ = self.sessions.push(session);
                Ok(body)
            },
            //should_quit,
        );
        let res_buf = match res_buf {
            Some(res_buf) => res_buf,
            None => return Err(ModuleError::Recoverable("timeout")),
        };
        let res_buf = res_buf?;

        let val: serde_json::Value = serde_json::from_str(&res_buf)?;
        let val = val
            .pointer("/value")
            .and_then(|x| x.as_str())
            .ok_or(anyhow::anyhow!("invalid json {}", val))?;

        Ok(String::from(val.trim()))
    }
}

#[no_mangle]
pub fn new_web_module(
    args: genvm_modules_interfaces::CtorArgs<'_>,
) -> anyhow::Result<Box<dyn genvm_modules_interfaces::Web + Send + Sync>> {
    let config: Config = serde_json::from_str(args.config)?;
    let host = config.host.clone();
    Ok(Box::new(Impl {
        sessions: crossbeam::queue::ArrayQueue::new(4),
        config,
        host,
    }))
}
