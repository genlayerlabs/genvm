use anyhow::Result;
use genvm_modules_common::*;

use serde_derive::Deserialize;

use std::ffi::CStr;

use genvm_modules_common::interfaces::web_functions_api;

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
    session_id: Option<String>,
    config: Config,
}

impl Drop for Impl {
    fn drop(&mut self) {
        match &self.session_id {
            Some(session_id) => {
                let _ = isahc::send(isahc::Request::delete(&format!("{}/session/{}", self.config.host, session_id)).body(()).unwrap());
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

impl Impl {
    fn init_session(&mut self) -> Result<()> {
        match &self.session_id {
            Some(_) => return Ok(()),
            None => {}
        }

        const INIT_REQUEST: &str = r#"{
                    "capabilities": {
                        "alwaysMatch": {
                            "browserName": "chrome",
                            "goog:chromeOptions": {
                                "args": ["--headless", "--no-sandbox", "--disable-dev-shm-usage"]
                            }
                        }
                    }
                }"#;

        let mut opened_session_res = isahc::send(isahc::Request::post(&format!("{}/session", &self.config.host))
            .header("Content-Type", "application/json; charset=utf-8")
            .body(INIT_REQUEST)?)?;
        let body = response::read(&mut opened_session_res)?;
        let val: serde_json::Value = serde_json::from_str(&body)?;
        let session_id = val
            .as_object()
            .and_then(|o| o.get_key_value("value"))
            .and_then(|val| val.1.as_object())
            .and_then(|o| o.get_key_value("sessionId"))
            .and_then(|val| val.1.as_str())
            .ok_or(anyhow::anyhow!("invalid json {}", val))?;
        self.session_id = Some(session_id.into());
        Ok(())
    }

    fn get_session(&self) -> Result<&str> {
        match &self.session_id {
            None => unreachable!(),
            Some(v) => Ok(v),
        }
    }

    fn try_new(args: &CtorArgs) -> Result<Self> {
        let conf: &str = args.config()?;
        let config: Config = serde_json::from_str(conf)?;
        Ok(Impl {
            session_id: None,
            config,
        })
    }

    fn get_webpage(&mut self, config: &CStr, url: &CStr) -> Result<String> {
        let config: GetWebpageConfig = serde_json::from_str(config.to_str()?)?;
        let url = url::Url::parse(url.to_str()?)?;

        self.init_session()?;
        let session_id = self.get_session()?;

        let req = serde_json::json!({
            "url": url.as_str()
        });
        let req = serde_json::to_string(&req)?;
        let mut res = isahc::send(isahc::Request::post(&format!("{}/session/{}/url", self.config.host, session_id))
            .header("Content-Type", "application/json; charset=utf-8")
            .body(req.as_bytes())?)?;
        let _ = response::read(&mut res)?;

        let script = match config.mode {
            GetWebpageConfigMode::html => {
                r#"{ "script": "return document.body.innerHTML", "args": [] }"#
            }
            GetWebpageConfigMode::text => {
                r#"{ "script": "return document.body.innerText.replace(/[\\s\\n]+/g, ' ')", "args": [] }"#
            }
        };

        let mut res = isahc::send(isahc::Request::post(&format!(
            "{}/session/{}/execute/sync",
            self.config.host, session_id
        ))
        .header("Content-Type", "application/json; charset=utf-8")
        .body(script)?)?;
        let res_buf = response::read(&mut res)?;

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
) -> interfaces::CStrResult {
    let ctx = get_ptr(ctx);
    let config = unsafe { CStr::from_ptr(config as *const i8) };
    let url = unsafe { CStr::from_ptr(url as *const i8) };
    ctx.get_webpage(config, url).into()
}
