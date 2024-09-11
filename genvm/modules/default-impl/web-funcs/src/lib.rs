use anyhow::Result;
use genvm_modules_common::*;

use serde_derive::Deserialize;

use std::{
    ffi::CStr,
    io::{stderr, Read, Write},
};

use genvm_modules_common::interfaces::web_functions_api;

genvm_modules_common::default_base_functions!(web_functions_api, Impl);

struct Impl {
    session_id: String,
    config: Config,
}

impl Drop for Impl {
    fn drop(&mut self) {
        let _ = ureq::delete(&format!("{}/session/{}", self.config.host, self.session_id)).call();
    }
}

#[derive(Deserialize)]
struct Config {
    host: String,
}

impl Impl {
    fn try_new(conf: &CStr) -> Result<Self> {
        let config: Config = serde_json::from_str(conf.to_str()?)?;
        let opened_session_res = ureq::post(&format!("{}/session", &config.host)).send_bytes(
            br#"{
            "capabilities": {
                "alwaysMatch": {
                    "browserName": "chrome",
                    "goog:chromeOptions": {
                        "args": ["--headless", "--no-sandbox", "--disable-dev-shm-usage"]
                    }
                }
            }
        }"#,
        )?;
        let status = opened_session_res.status();
        let body = opened_session_res
            .into_string()
            .unwrap_or(r#"{"value":{"error":"can't read body from genvm"}}"#.into());
        if status != 200 {
            return Err(anyhow::anyhow!("couldn't initialize {}", body));
        }
        let val: serde_json::Value = serde_json::from_str(&body)?;
        let session_id = val
            .as_object()
            .and_then(|o| o.get_key_value("value"))
            .and_then(|val| val.1.as_object())
            .and_then(|o| o.get_key_value("sessionId"))
            .and_then(|val| val.1.as_str())
            .ok_or(anyhow::anyhow!("invalid json {}", val))?;
        Ok(Impl {
            session_id: String::from(session_id),
            config,
        })
    }

    fn get_webpage(&mut self, _config: &CStr, url: &CStr) -> Result<String> {
        let url = url::Url::parse(url.to_str()?)?;

        let req = serde_json::Value::Object(serde_json::Map::from_iter(
            [("url".into(), url.as_str().into())].into_iter(),
        ));
        let req = serde_json::to_string(&req)?;
        let res = ureq::post(&format!(
            "{}/session/{}/url",
            self.config.host, &self.session_id
        ))
        .send_bytes(req.as_bytes())?;
        if res.status() != 200 {
            return Err(anyhow::anyhow!("can't get webpage {:?}", res));
        }

        let script = r#"{ "script": "return document.body.innerText.replace(/[\\s\\n]+/g, ' ')", "args": [] }"#;

        let res = ureq::post(&format!(
            "{}/session/{}/execute/sync",
            self.config.host, &self.session_id
        ))
        .send_bytes(script.as_bytes())?;
        if res.status() != 200 {
            return Err(anyhow::anyhow!("can't get webpage contents {:?}", res));
        }

        let encoding = encoding_rs::Encoding::for_label(res.charset().as_bytes())
            .unwrap_or(encoding_rs::UTF_8);

        let mut res_buf = String::new();
        let res_reader = res.into_reader();
        let mut res_reader = encoding_rs_io::DecodeReaderBytesBuilder::new()
            .encoding(Some(encoding))
            .build(res_reader);
        let _ = res_reader.read_to_string(&mut res_buf)?;

        let val: serde_json::Value = serde_json::from_str(&res_buf)?;
        let val = val
            .as_object()
            .and_then(|x| x.get_key_value("value"))
            .and_then(|x| x.1.as_str())
            .ok_or(anyhow::anyhow!("invalid json {}", val))?;

        Ok(String::from(val))
    }
}

fn errored_res(code: i32, err: anyhow::Error) -> interfaces::CStrResult {
    let _ = stderr()
        .lock()
        .write_fmt(format_args!("{} err: {:?}", env!("CARGO_PKG_NAME"), err));
    return interfaces::CStrResult {
        str: std::ptr::null(),
        err: code,
    };
}

#[no_mangle]
pub extern "C" fn get_webpage(
    ctx: *const (),
    _gas: &mut u64,
    config: *const u8,
    url: *const u8,
) -> interfaces::CStrResult {
    let ctx = get_ptr(ctx);
    let config = unsafe { CStr::from_ptr(config as *const i8) };
    let url = unsafe { CStr::from_ptr(url as *const i8) };
    match ctx.get_webpage(config, url) {
        Err(e) => errored_res(1, e),
        Ok(s) => ok_str_result(&s),
    }
}

fn ok_str_result(s: &str) -> interfaces::CStrResult {
    unsafe {
        let res = libc::malloc(s.len() + 1) as *mut u8;
        *res.add(s.len()) = 0;
        libc::memcpy(
            res as *mut std::ffi::c_void,
            s.as_ptr() as *const std::ffi::c_void,
            s.len(),
        );
        interfaces::CStrResult {
            str: res as *const u8,
            err: 0,
        }
    }
}
