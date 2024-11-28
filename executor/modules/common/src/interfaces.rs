#![allow(unused_parens)]

use serde::{Deserialize, Serialize};
//use serde_derive::{Deserialize, Serialize};

macro_rules! impl_create_trait_fn {
    ($name:ident : fn ($($arg_name:ident: $arg_t:ty),*) -> $ret_t:tt ;) => {
        fn $name(&mut self, $( $arg_name : $arg_t ),*) -> $ret_t;
    };
}

macro_rules! create_trait {
    (($name:ident, $ver:expr) { $($fn_name:ident : fn ($($arg_name:ident: $arg_t:ty),*) -> $ret_t:tt ;)* }) => {
        pub mod $name {
            use crate::Version;
            pub const VERSION: Version = $ver;
            pub const NAME: &str = std::stringify!($name);
            pub trait Trait: Send + Sync {
                $(impl_create_trait_fn!($fn_name : fn ($($arg_name: $arg_t),*) -> $ret_t ;);)*
            }

            pub struct Methods;
        }
    };
}

#[repr(C)]
pub struct BytesResult {
    pub ptr: *const u8,
    pub len: u32,
}

#[derive(serde_derive::Serialize, serde_derive::Deserialize)]
pub enum ModuleResult<T> {
    Success(T),
    Error(String),
}

impl<T> From<anyhow::Result<T>> for ModuleResult<T> {
    fn from(value: anyhow::Result<T>) -> Self {
        match value {
            Ok(t) => ModuleResult::Success(t),
            Err(e) => ModuleResult::Error(format!("{e:?}")),
        }
    }
}

pub fn serialize_result<T: Serialize>(res: anyhow::Result<T>) -> BytesResult {
    let as_mod_res = ModuleResult::from(res);
    as_mod_res.to_bytes().unwrap()
}

impl<T> ModuleResult<T> {
    pub fn into_anyhow(self) -> anyhow::Result<T> {
        match self {
            ModuleResult::Success(val) => Ok(val),
            ModuleResult::Error(message) => Err(anyhow::format_err!("{}", message)),
        }
    }
}

impl<T: Serialize> ModuleResult<T> {
    pub fn to_bytes(&self) -> anyhow::Result<BytesResult> {
        let vec = rmp_serde::to_vec(&self)?;
        let len_u32: u32 = vec.len().try_into()?;
        let res_arr = unsafe { libc::malloc(vec.len()) } as *mut u8;
        anyhow::ensure!(res_arr != std::ptr::null_mut());
        let res = unsafe { std::slice::from_raw_parts_mut(res_arr, vec.len()) };
        res.copy_from_slice(&vec);
        Ok(BytesResult {
            ptr: res_arr,
            len: len_u32,
        })
    }
}

impl<'a, T: Deserialize<'a>> ModuleResult<T> {
    pub fn from_bytes(
        bytes: BytesResult,
        free: impl FnOnce(*const u8) -> (),
    ) -> anyhow::Result<Self> {
        let slice = unsafe { std::slice::from_raw_parts(bytes.ptr, bytes.len as usize) };
        let res: anyhow::Result<Self> = rmp_serde::from_slice(slice).map_err(Into::into);
        free(bytes.ptr);
        res
    }
}

#[macro_export]
macro_rules! WebFunctionsApiFns {
    ($cb:path[$($args:tt),*]) => {
        $cb!(($($args),*) {
            free_str: fn(data: *const u8) -> ();
            get_webpage: fn(gas: &mut u64, config: *const u8, url: *const u8) -> ($crate::interfaces::BytesResult); // ModuleResult<String>
        });
    };
}
WebFunctionsApiFns!(create_trait[web_functions_api, (Version { major: 0, minor: 0 })]);

#[macro_export]
macro_rules! LLMFunctionsApiFns {
    ($cb:path[$($args:tt),*]) => {
        $cb!(($($args),*) {
            free_str: fn(data: *const u8) -> ();
            exec_prompt: fn(gas: &mut u64, config: *const u8, prompt: *const u8) -> ($crate::interfaces::BytesResult); // ModuleResult<String>
            exec_prompt_id: fn(gas: &mut u64, id: u8, vars: *const u8) -> ($crate::interfaces::BytesResult); // ModuleResult<String>
            eq_principle_prompt: fn(gas: &mut u64, id: u8, vars: *const u8) -> ($crate::interfaces::BytesResult); // ModuleResult<bool>
        });
    };
}
LLMFunctionsApiFns!(create_trait[llm_functions_api, (Version { major: 0, minor: 0 })]);

#[no_mangle]
pub extern "C-unwind" fn free_str(_ctx: *const (), data: *const u8) -> () {
    unsafe { libc::free(data as *mut std::ffi::c_void) };
}
