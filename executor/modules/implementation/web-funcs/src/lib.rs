use anyhow::Context;
use genvm_modules_impl_common::*;
use genvm_modules_interfaces::{CancellationToken, ModuleError, ModuleResult};
use serde_derive::Deserialize;

use std::{future::Future, mem::swap, pin::Pin, sync::Arc};

struct SessionData {
    id: Box<str>,
    host: Arc<str>,
}

impl SessionDrop for SessionData {
    fn has_drop_session() -> bool {
        true
    }

    fn drop_session(
        client: reqwest::Client,
        data: &mut SessionData,
    ) -> Pin<Box<dyn Future<Output = ()> + Send + Sync>> {
        let mut id = Box::from("");
        swap(&mut data.id, &mut id);
        let host = data.host.clone();

        Box::pin(async move {
            let _ = client
                .delete(format!("{}/session/{}", host, id))
                .send()
                .await;
        })
    }
}

struct Impl {
    sessions: SessionPool<SessionData>,
    config: Config,
    host: Arc<str>,
    cancellation: Arc<CancellationToken>,
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

fn no_wait() -> ParsedDuration {
    ParsedDuration(tokio::time::Duration::ZERO)
}

#[derive(Deserialize)]
struct GetWebpageConfig {
    mode: GetWebpageConfigMode,
    #[serde(default = "no_wait")]
    wait_after_loaded: ParsedDuration,
}

unsafe impl Send for Impl {}

impl Impl {
    async fn get_session(&self) -> ModuleResult<Box<Session<SessionData>>> {
        if let Some(s) = self.sessions.get() {
            return Ok(s);
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

        let client = reqwest::Client::new();
        let opened_session_res = client
            .post(format!("{}/session", &self.config.host))
            .header("Content-Type", "application/json; charset=utf-8")
            .body(INIT_REQUEST)
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
            .ok_or(anyhow::anyhow!("invalid json {}", val))?;

        Ok(Box::new(Session {
            client,
            data: SessionData {
                id: Box::from(session_id),
                host: self.host.clone(),
            },
        }))
    }
}

impl Impl {
    async fn get_webpage(
        &self,
        config: String,
        url: String,
        session: &mut Session<SessionData>,
    ) -> ModuleResult<String> {
        let config: GetWebpageConfig =
            make_error_recoverable(serde_json::from_str(&config), "invalid config")?;
        let url = url::Url::parse(&url)?;
        if url.scheme() == "file" {
            return Err(ModuleError::Recoverable("file scheme is forbidden"));
        }

        if url.host_str() != Some("genvm-test") {
            const ALLOWED_PORTS: &[Option<u16>] = &[None, Some(80), Some(443)];
            if !ALLOWED_PORTS.contains(&url.port()) {
                return Err(ModuleError::Recoverable("port is forbidden"));
            }
        }

        let req_body = serde_json::json!({
            "url": url.as_str()
        });
        let req_body = serde_json::to_string(&req_body)?;
        let req = session
            .client
            .post(format!(
                "{}/session/{}/url",
                self.config.host, session.data.id
            ))
            .header("Content-Type", "application/json; charset=utf-8")
            .body(req_body.clone());

        log::info!(request:? = req, body = req_body; "sending request");

        let res = req.send().await?;
        let res = res.error_for_status()?;
        std::mem::drop(res);

        match config.wait_after_loaded {
            ParsedDuration(tokio::time::Duration::ZERO) => {}
            ParsedDuration(x) => {
                log::trace!(duration:? = x; "sleeping to allow page to load");
                tokio::time::sleep(x).await
            }
        }

        let script = match config.mode {
            GetWebpageConfigMode::html => {
                r#"{ "script": "return document.body.innerHTML", "args": [] }"#
            }
            GetWebpageConfigMode::text => {
                r#"{ "script": "return document.body.innerText.replace(/[\\s\\n]+/g, ' ')", "args": [] }"#
            }
        };

        let req = session
            .client
            .post(format!(
                "{}/session/{}/execute/sync",
                self.config.host, session.data.id
            ))
            .header("Content-Type", "application/json; charset=utf-8")
            .body(script);
        log::debug!(request:? = req, body = script; "getting web page data");

        let res = req.send().await?;

        let res = res.error_for_status()?;

        let body = res.text().await?;

        let res_buf = body;

        let val: serde_json::Value = serde_json::from_str(&res_buf)?;
        let val = val
            .pointer("/value")
            .and_then(|x| x.as_str())
            .ok_or(anyhow::anyhow!("invalid json {}", val))?;

        Ok(String::from(val.trim()))
    }
}

struct Proxy(Arc<Impl>);

#[async_trait::async_trait]
impl genvm_modules_interfaces::Web for Proxy {
    fn get_webpage(
        &self,
        config: String,
        url: String,
    ) -> tokio::task::JoinHandle<anyhow::Result<Box<[u8]>>> {
        async fn forward(zelf: Arc<Impl>, config: String, url: String) -> ModuleResult<String> {
            let mut session = zelf.get_session().await?;
            tokio::select! {
                res = zelf.get_webpage(config, url, &mut session) => {
                    zelf.sessions.retn(session);
                    res
                }
                _ = zelf.cancellation.chan.closed() => {
                    Err(ModuleError::Fatal(anyhow::anyhow!("timeout")))
                }
            }
        }
        tokio::spawn(genvm_modules_interfaces::module_result_to_future(forward(
            self.0.clone(),
            config,
            url,
        )))
    }
}

#[no_mangle]
pub fn new_web_module(
    args: genvm_modules_interfaces::CtorArgs,
) -> anyhow::Result<Box<dyn genvm_modules_interfaces::Web + Send + Sync>> {
    let config: Config = serde_yaml::from_value(args.config)?;
    let host = config.host.clone();
    Ok(Box::new(Proxy(Arc::new(Impl {
        sessions: SessionPool::new(),
        config,
        host,
        cancellation: args.cancellation,
    }))))
}
