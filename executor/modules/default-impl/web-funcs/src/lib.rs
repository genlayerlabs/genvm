use anyhow::Result;
use genvm_modules_common::*;

use serde_derive::Deserialize;

use std::{ffi::CStr, sync::Arc};

use genvm_modules_common::interfaces::web_functions_api;
use genvm_modules_impl_common::run_with_termination;

use crate::interfaces::RecoverableError;

mod response;

struct MyAlloc;

unsafe impl std::alloc::GlobalAlloc for MyAlloc {
    unsafe fn alloc(&self, layout: std::alloc::Layout) -> *mut u8 {
        libc::malloc(layout.size()) as *mut u8
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: std::alloc::Layout) {
        libc::free(ptr as *mut std::ffi::c_void)
    }
}

#[global_allocator]
static A: MyAlloc = MyAlloc;

genvm_modules_common::default_base_functions!(web_functions_api, Impl);

struct Impl {
    session_id: Option<Arc<str>>,
    config: Config,
    should_quit: *mut u32,
}

impl Drop for Impl {
    fn drop(&mut self) {
        match &self.session_id {
            Some(session_id) => {
                let mut builder =
                    isahc::Request::delete(&format!("{}/session/{}", self.config.host, session_id));
                if unsafe { std::sync::atomic::AtomicU32::from_ptr(self.should_quit) }
                    .load(std::sync::atomic::Ordering::SeqCst)
                    != 0
                {
                    // FIXME for some reason webdriver blocks on delete
                    use isahc::config::Configurable;
                    builder = builder.timeout(std::time::Duration::from_millis(2));
                }
                let _ = isahc::send(builder.body(()).unwrap());
            }
            None => {}
        }
    }
}

#[derive(Deserialize)]
struct Config {
    host: String,
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
    fn get_session(&mut self) -> Result<Arc<str>> {
        if self.session_id.is_none() {
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

            let mut opened_session_res = isahc::send(
                isahc::Request::post(&format!("{}/session", &self.config.host))
                    .header("Content-Type", "application/json; charset=utf-8")
                    .body(INIT_REQUEST)?,
            )?;
            let body = response::read(&mut opened_session_res)?;
            let val: serde_json::Value = serde_json::from_str(&body)?;
            let session_id = val
                .as_object()
                .and_then(|o| o.get_key_value("value"))
                .and_then(|val| val.1.as_object())
                .and_then(|o| o.get_key_value("sessionId"))
                .and_then(|val| val.1.as_str())
                .ok_or(anyhow::anyhow!("invalid json {}", val))?;
            self.session_id = Some(Arc::from(session_id));
        }

        match &self.session_id {
            Some(v) => Ok(v.clone()),
            None => unreachable!(),
        }
    }

    fn try_new(args: &CtorArgs) -> Result<Self> {
        let conf: &str = args.config()?;
        let config: Config = serde_json::from_str(conf)?;
        Ok(Impl {
            session_id: None,
            config,
            should_quit: args.should_quit,
        })
    }

    fn get_webpage(&mut self, config: &CStr, url: &CStr) -> Result<String> {
        let config: GetWebpageConfig = serde_json::from_str(config.to_str()?)?;
        let url = url::Url::parse(url.to_str()?)?;
        if url.scheme() == "file" {
            return Err(RecoverableError(anyhow::anyhow!("file scheme is forbidden")).into());
        }

        if url.host_str() != Some("genvm-test") {
            const ALLOWED_PORTS: &[Option<u16>] = &[None, Some(80), Some(443)];
            if !ALLOWED_PORTS.contains(&url.port()) {
                return Err(RecoverableError(anyhow::anyhow!(
                    "port {:?} is forbidden",
                    url.port()
                ))
                .into());
            }
        }

        let should_quit = self.should_quit;
        let res_buf: Option<anyhow::Result<String>> = run_with_termination(
            async move {
                let session_id = self.get_session()?;

                let client = reqwest::Client::new();
                let req = serde_json::json!({
                    "url": url.as_str()
                });
                let req = serde_json::to_string(&req)?;
                let res = client
                    .post(&format!("{}/session/{}/url", self.config.host, session_id))
                    .header("Content-Type", "application/json; charset=utf-8")
                    .body(req)
                    .send()
                    .await?;
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

                let res = client
                    .post(&format!(
                        "{}/session/{}/execute/sync",
                        self.config.host, session_id
                    ))
                    .header("Content-Type", "application/json; charset=utf-8")
                    .body(script)
                    .send()
                    .await?;

                let res = res.error_for_status()?;

                let body = res.text().await?;
                Ok(body)
            },
            should_quit,
        );
        let res_buf = match res_buf {
            Some(res_buf) => res_buf,
            None => return Err(RecoverableError(anyhow::anyhow!("timeout")).into()),
        };
        let res_buf = res_buf?;

        let val: serde_json::Value = serde_json::from_str(&res_buf)?;
        let val = val
            .as_object()
            .and_then(|x| x.get_key_value("value"))
            .and_then(|x| x.1.as_str())
            .ok_or(anyhow::anyhow!("invalid json {}", val))?;

        Ok(String::from(val.trim()))
    }
}

#[no_mangle]
pub extern "C-unwind" fn get_webpage(
    ctx: *const (),
    _gas: &mut u64,
    config: *const u8,
    url: *const u8,
) -> interfaces::BytesResult {
    let ctx = get_ptr(ctx);
    let config = unsafe { CStr::from_ptr(config as *const std::ffi::c_char) };
    let url = unsafe { CStr::from_ptr(url as *const std::ffi::c_char) };
    let res = ctx.get_webpage(config, url);
    interfaces::serialize_result(res)
}
