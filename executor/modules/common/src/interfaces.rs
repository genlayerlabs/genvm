#![allow(unused_parens)]

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
pub struct CStrResult {
    pub str: *const u8,
    pub err: i32,
}

#[repr(C)]
pub struct BoolResult {
    pub res: bool,
    pub err: i32,
}

impl<S> From<anyhow::Result<S>> for CStrResult
where
    S: AsRef<str>,
{
    fn from(value: anyhow::Result<S>) -> Self {
        match value {
            Ok(v) => CStrResult {
                str: crate::str_to_shared(v.as_ref()),
                err: 0,
            },
            Err(e) => {
                eprintln!("Module error {}", &e);
                CStrResult {
                    str: std::ptr::null(),
                    err: 1,
                }
            }
        }
    }
}

impl From<anyhow::Result<bool>> for BoolResult {
    fn from(value: anyhow::Result<bool>) -> Self {
        match value {
            Ok(v) => BoolResult { res: v, err: 0 },
            Err(e) => {
                eprintln!("Module error {}", &e);
                BoolResult { res: false, err: 1 }
            }
        }
    }
}

#[macro_export]
macro_rules! WebFunctionsApiFns {
    ($cb:path[$($args:tt),*]) => {
        $cb!(($($args),*) {
            get_webpage: fn(gas: &mut u64, config: *const u8, url: *const u8) -> ($crate::interfaces::CStrResult);
        });
    };
}
WebFunctionsApiFns!(create_trait[web_functions_api, (Version { major: 0, minor: 0 })]);

#[macro_export]
macro_rules! LLMFunctionsApiFns {
    ($cb:path[$($args:tt),*]) => {
        $cb!(($($args),*) {
            exec_prompt: fn(gas: &mut u64, config: *const u8, prompt: *const u8) -> ($crate::interfaces::CStrResult);
            eq_principle_prompt: fn(gas: &mut u64, config: *const u8) -> ($crate::interfaces::BoolResult);
        });
    };
}
LLMFunctionsApiFns!(create_trait[llm_functions_api, (Version { major: 0, minor: 0 })]);
