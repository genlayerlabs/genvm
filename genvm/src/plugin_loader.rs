pub fn default_plugin_path() -> std::io::Result<std::path::PathBuf> {
    let mut buf = std::env::current_exe()?;
    buf.pop();
    buf.pop();
    buf.push("lib");
    buf.push("genvm-modules");
    Ok(buf)
}

macro_rules! vtable_decl_fields {
    ($($name:ident : fn ($($arg_name:ident: $arg_t:ty),*) -> $ret_t:tt ;)*) => {
        struct Impl {
            #[allow(dead_code)]
            lib: std::sync::Arc<libloading::Library>,
            ctx: *const (),
            dtor: libloading::os::unix::Symbol<unsafe extern fn(*const () ) -> ()>,
            $($name: libloading::os::unix::Symbol<unsafe extern fn(ctx: *const (), $( $arg_name : $arg_t ),*) -> $ret_t>,)*
        }
    };
}

macro_rules! impl_create_trait_fn {
    ($name:ident : fn ($($arg_name:ident: $arg_t:ty),*) -> $ret_t:tt ;) => {
        fn $name(&mut self, $( $arg_name : $arg_t ),*) -> $ret_t {
            return unsafe { (self.$name)(self.ctx, $($arg_name),*) };
        }
    };
}

macro_rules! unwrap_ctor_inits {
    ($lib:ident, $ctx:ident, $dtor:ident, $($name:ident : fn ($($arg_name:ident: $arg_t:ty),*) -> $ret_t:tt ;)*) => (
        Impl {
            lib: $lib.clone(),
            ctx: $ctx,
            dtor: $dtor,
            $($name: get_sym($lib.get(stringify!($name).as_bytes())?),)*
        }
    );
}

macro_rules! create_trait {
    (($name:ident) { $($fn_name:ident : fn ($($arg_name:ident: $arg_t:ty),*) -> $ret_t:tt ;)* }) => {
        pub mod $name {
            pub trait Loader {
                fn load_from_lib(path: &std::path::Path, name: &str) -> anyhow::Result<Box<dyn genvm_modules_common::interfaces::$name::Trait>>;
            }
        }
        impl $name::Loader for genvm_modules_common::interfaces::$name::Methods {
            fn load_from_lib(path: &std::path::Path, name: &str) -> anyhow::Result<Box<dyn genvm_modules_common::interfaces::$name::Trait>> {
                use anyhow::Context;
                let name = libloading::library_filename(name);
                let final_path = path.join(name);
                let lib = unsafe {
                    let lib = libloading::Library::new(&final_path).with_context(|| format!("loading lib {:?}", &final_path))?;
                    let lib = std::sync::Arc::new(lib);
                    let checker: libloading::Symbol<fn(v: *const genvm_modules_common::Version) -> bool> = lib.get(b"check_version")?;
                    let ver = genvm_modules_common::interfaces::$name::VERSION;
                    if !checker(&ver) {
                        return Err(anyhow::anyhow!("Version didn't amtch"));
                    }
                    lib
                };

                vtable_decl_fields!($($fn_name : fn ($($arg_name : $arg_t),*) -> $ret_t ;)*);

                impl std::ops::Drop for Impl {
                    fn drop(&mut self) {
                        unsafe { (self.dtor)(self.ctx); }
                    }
                }

                unsafe impl Sync for Impl {}
                unsafe impl Send for Impl {}

                impl genvm_modules_common::interfaces::$name::Trait for Impl {
                    $(impl_create_trait_fn!($fn_name : fn ($($arg_name: $arg_t),*) -> $ret_t ;);)*
                }

                fn get_sym<T>(f: libloading::Symbol<T>) -> libloading::os::unix::Symbol<T> {
                    return unsafe { f.into_raw() };
                }

                unsafe {
                    let ctor: libloading::Symbol<unsafe extern fn() -> *mut()> = lib.get(b"ctor")?;
                    let dtor: libloading::os::unix::Symbol<unsafe extern fn(*const () ) -> ()> = get_sym(lib.get(b"dtor")?);
                    let dtor_cop = dtor.clone();
                    let ctx = ctor();
                    let f = || {
                        Ok(unwrap_ctor_inits!(lib, ctx, dtor, $($fn_name : fn ($($arg_name : $arg_t),*) -> $ret_t ;)*))
                    };
                    match f() {
                        Ok(x) => Ok(Box::new(x)),
                        Err(e) => {
                            dtor_cop(ctx);
                            Err(e)
                        }
                    }
                }
            }
        }
    };
}

genvm_modules_common::WebFunctionsApiFns!(create_trait[web_functions_api]);
genvm_modules_common::LLMFunctionsApiFns!(create_trait[llm_functions_api]);
