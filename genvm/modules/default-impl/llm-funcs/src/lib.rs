use anyhow::Result;
use genvm_modules_common::*;

use std::{
    ffi::CStr,
    io::{stderr, Read, Write},
};

use genvm_modules_common::interfaces::web_functions_api;

genvm_modules_common::default_base_functions!(web_functions_api, Impl);

struct Impl {}

impl Drop for Impl {
    fn drop(&mut self) {
    }
}

impl Impl {
    fn try_new(_conf: &CStr) -> Result<Self> {
        Ok(Impl {})
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
        interfaces::CStrResult {
            str: str_to_shared(s),
            err: 0,
        }
    }
}
