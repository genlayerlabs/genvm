use anyhow::Result;
use genvm_modules_common::*;

use std::{
    ffi::CStr,
    io::{stderr, Read, Write},
};

#[no_mangle]
pub extern "C" fn check_version(v: *const Version) -> bool {
    return genvm_modules_common::interfaces::nondet_functions_api::VERSION == unsafe { *v };
}

#[no_mangle]
pub extern "C" fn ctor() -> *const () {
    unsafe fn imp() -> Result<*const ()> {
        let layout = std::alloc::Layout::new::<std::mem::MaybeUninit<Impl>>();
        let res: *mut std::mem::MaybeUninit<Impl> = std::alloc::alloc(layout).cast();
        let opened_session_res = ureq::post("http://127.0.0.1:4444/session").send_bytes(
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
        (*res).write(Impl {
            session_id: String::from(session_id),
        });
        return Ok(res as *const ());
    }
    match unsafe { imp() } {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{}\nbt: {}", e, e.backtrace());
            panic!("couldn't initialize module");
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn dtor(ptr: *const ()) {
    let ctx = get_ptr(ptr);
    std::ptr::drop_in_place(ctx);
    let layout = std::alloc::Layout::new::<std::mem::MaybeUninit<Impl>>();
    std::alloc::dealloc(ptr as *mut u8, layout);
}

fn get_ptr(ptr: *const ()) -> &'static mut Impl {
    unsafe {
        let ptr = ptr as *mut Impl;
        return &mut *ptr;
    }
}

struct Impl {
    session_id: String,
}

impl Drop for Impl {
    fn drop(&mut self) {
        let _ = ureq::delete(&format!(
            "http://127.0.0.1:4444/session/{}",
            self.session_id
        ))
        .call();
    }
}

impl Impl {
    fn get_webpage(&mut self, _config: &CStr, url: &CStr) -> Result<String> {
        let url = url::Url::parse(url.to_str()?)?;

        let req = serde_json::Value::Object(serde_json::Map::from_iter(
            [("url".into(), url.as_str().into())].into_iter(),
        ));
        let req = serde_json::to_string(&req)?;
        let res = ureq::post(&format!(
            "http://127.0.0.1:4444/session/{}/url",
            &self.session_id
        ))
        .send_bytes(req.as_bytes())?;
        if res.status() != 200 {
            return Err(anyhow::anyhow!("can't get webpage {:?}", res));
        }

        let script = r#"{ "script": "return document.body.innerText.replace(/[\\s\\n]+/g, ' ')", "args": [] }"#;

        let res = ureq::post(&format!(
            "http://127.0.0.1:4444/session/{}/execute/sync",
            &self.session_id
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

    fn call_llm(&mut self, _config: &CStr, _prompt: &CStr) -> Result<String> {
        todo!()
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

#[no_mangle]
pub extern "C" fn call_llm(
    ctx: *const (),
    _gas: &mut u64,
    config: *const u8,
    prompt: *const u8,
) -> interfaces::CStrResult {
    let ctx = get_ptr(ctx);
    let config = unsafe { CStr::from_ptr(config as *const i8) };
    let prompt = unsafe { CStr::from_ptr(prompt as *const i8) };
    match ctx.call_llm(config, prompt) {
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
