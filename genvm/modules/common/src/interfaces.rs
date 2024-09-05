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

#[macro_export]
macro_rules! NondetFunctionsApiFns {
    ($cb:path[$($args:tt),*]) => {
        $cb!(($($args),*) {
            get_webpage: fn(gas: &mut u64, config: *const u8, url: *const u8) -> ($crate::interfaces::CStrResult);
            call_llm: fn(gas: &mut u64, config: *const u8, prompt: *const u8) -> ($crate::interfaces::CStrResult);
        });
    };
}

NondetFunctionsApiFns!(create_trait[nondet_functions_api, (Version { major: 0, minor: 0 })]);
