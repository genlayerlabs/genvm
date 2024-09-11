use anyhow::Result;
use genvm_modules_common::*;

use std::ffi::CStr;

use genvm_modules_common::interfaces::web_functions_api;

genvm_modules_common::default_base_functions!(web_functions_api, Impl);

struct Impl {}

impl Drop for Impl {
    fn drop(&mut self) {}
}

impl Impl {
    fn try_new(_conf: &CStr) -> Result<Self> {
        Ok(Impl {})
    }

    fn call_llm(&mut self, _config: &CStr, _prompt: &CStr) -> Result<String> {
        todo!()
    }
}

#[no_mangle]
pub extern "C-unwind" fn call_llm(
    ctx: *const (),
    _gas: &mut u64,
    config: *const u8,
    prompt: *const u8,
) -> interfaces::CStrResult {
    let ctx = get_ptr(ctx);
    let config = unsafe { CStr::from_ptr(config as *const i8) };
    let prompt = unsafe { CStr::from_ptr(prompt as *const i8) };
    ctx.call_llm(config, prompt).into()
}
