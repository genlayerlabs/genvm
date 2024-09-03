use genvm_modules_common::*;

use std::ffi::CStr;

#[no_mangle]
pub extern "C" fn check_version(v: *const Version) -> bool {
    return genvm_modules_common::interfaces::nondet_functions_api::VERSION == unsafe { *v };
}

#[no_mangle]
pub unsafe extern "C" fn ctor() -> *const () {
    let layout = std::alloc::Layout::new::<std::mem::MaybeUninit<Impl>>();
    let res: *mut std::mem::MaybeUninit<Impl> = std::alloc::alloc(layout).cast();
    (*res).write(Impl {});
    return res as *const ();
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

struct Impl;

impl Drop for Impl {
    fn drop(&mut self) {
    }
}

impl Impl {
    fn get_webpage(&mut self, url: &CStr) -> anyhow::Result<String> {
        let url = url.to_str()?;
        let body = ureq::get(url).call()?.into_string()?;
        Ok(body)
    }

    fn call_llm(&mut self, _prompt: &CStr) -> anyhow::Result<String> {
        todo!()
    }
}

fn errored_res(code: i32) -> interfaces::CStrResult {
    return interfaces::CStrResult {
        str: std::ptr::null(),
        err: code,
    }
}

#[no_mangle]
pub extern "C" fn get_webpage(
    ctx: *const (),
    gas: &mut u64,
    url: *const u8,
) -> interfaces::CStrResult {
    let ctx = get_ptr(ctx);
    let url = unsafe { CStr::from_ptr(url as *const i8) };
    match ctx.get_webpage(url) {
        Err(_) => errored_res(1),
        Ok(s) => ok_str_result(&s),
    }
}

#[no_mangle]
pub extern "C" fn call_llm(
    ctx: *const (),
    gas: &mut u64,
    prompt: *const u8,
) -> interfaces::CStrResult {
    let ctx = get_ptr(ctx);
    let prompt = unsafe { CStr::from_ptr(prompt as *const i8) };
    match ctx.call_llm(prompt) {
        Err(_) => errored_res(1),
        Ok(s) => ok_str_result(&s),
    }
}

fn ok_str_result(s: &str) -> interfaces::CStrResult {
    unsafe {
        let res = libc::malloc(s.len() + 1) as *mut u8;
        *res.add(s.len()) = 0;
        libc::memcpy(res as *mut std::ffi::c_void, s.as_ptr() as *const std::ffi::c_void, s.len());
        interfaces::CStrResult {
            str: res as *const u8,
            err: 0,
        }
    }
}
