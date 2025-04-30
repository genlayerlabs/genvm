use std::{str::FromStr, sync::Arc};

use crate::{calldata, vm};

pub mod base;
mod gl_call;
mod common;
pub mod genlayer_sdk;
pub mod preview1;

pub struct Context {
    vfs: common::VFS,
    pub preview1: preview1::Context,
    pub genlayer_sdk: genlayer_sdk::Context,
}

impl Context {
    pub fn new(data: genlayer_sdk::SingleVMData, shared_data: Arc<vm::SharedData>) -> anyhow::Result<Self> {
        let as_value = calldata::to_value(&data.message_data)?;
        let as_bytes = calldata::encode(&as_value);
        Ok(Self {
            vfs: common::VFS::new(as_bytes),
            preview1: preview1::Context::new(data.message_data.datetime, data.conf),
            genlayer_sdk: genlayer_sdk::Context::new(data, shared_data),
        })
    }
}

#[cfg(any())]
fn add_to_linker_sync_dlsym<T: Send + 'static>(
    linker: &mut wasmtime::Linker<T>,
    linker_shared: Arc<Mutex<wasmtime::Linker<T>>>,
) -> anyhow::Result<()> {
    use core::str;

    linker.func_wrap(
        "genlayer_dl",
        "dlsym",
        move |mut caller: wasmtime::Caller<'_, _>,
              mod_name: u32,
              mod_name_len: u32,
              func_name: u32,
              func_name_len: u32|
              -> wiggle::anyhow::Result<i32> {
            let export = caller.get_export("memory");
            let mem = match &export {
                Some(wiggle::wasmtime_crate::Extern::Memory(m)) => {
                    let (mem, _ctx) = m.data_and_store_mut(&mut caller);
                    wiggle::GuestMemory::Unshared(mem)
                }
                Some(wiggle::wasmtime_crate::Extern::SharedMemory(m)) => {
                    wiggle::GuestMemory::Shared(m.data())
                }
                _ => {
                    return anyhow::__private::Err({
                        let error = anyhow::__private::format_err(anyhow::__private::format_args!(
                            "missing required memory export"
                        ));
                        error
                    })
                }
            };

            let mod_name_ptr: wiggle::GuestPtr<[u8]> =
                wiggle::GuestPtr::new(mod_name).as_array(mod_name_len);
            let func_name_ptr: wiggle::GuestPtr<[u8]> =
                wiggle::GuestPtr::new(func_name).as_array(func_name_len);

            let mod_name = Vec::from_iter(mem.as_cow(mod_name_ptr)?.into_iter().cloned());
            let mod_name = str::from_utf8(&mod_name)?;

            let mod_name = match mod_name.rfind('/') {
                Some(i) => &mod_name[i + 1..],
                _ => mod_name,
            };

            if mod_name.is_empty() {
                anyhow::bail!("can't load from unnamed");
            }

            let func_name = Vec::from_iter(mem.as_cow(func_name_ptr)?.into_iter().cloned());
            let func_name = str::from_utf8(&func_name)?;

            log::trace!(target: "rt", module = mod_name, function = func_name; "dlsym called");

            let linker_shared = linker_shared.clone();
            let Ok(ref mut linker) = linker_shared.lock() else {
                panic!();
            };

            let fn_exported = linker
                .get(&mut caller, mod_name, func_name)
                .ok_or_else(|| anyhow::anyhow!("function entity not found"))?
                .into_func()
                .ok_or_else(|| anyhow::anyhow!("found entity is not a function"))?;

            let table = caller
                .get_export("__indirect_function_table")
                .unwrap()
                .into_table()
                .ok_or_else(|| anyhow::anyhow!("no __indirect_function_table"))?;
            let res = table.grow(&mut caller, 1, fn_exported.into())?;
            Ok(res.try_into()?)
        },
    )?;
    Ok(())
}

pub(super) fn add_to_linker_sync<T: Send + 'static>(
    linker: &mut wasmtime::Linker<T>,
    _linker_shared: Arc<tokio::sync::Mutex<wasmtime::Linker<T>>>,
    f: impl Fn(&mut T) -> &mut Context + Copy + Send + Sync + 'static,
) -> anyhow::Result<()> {
    #[derive(Clone, Copy)]
    struct Fwd<F>(F);

    impl<T, F> preview1::AddToLinkerFn<T> for Fwd<F>
    where
        F: Fn(&mut T) -> &mut Context + Copy + Send + Sync + 'static,
    {
        fn call<'a>(&self, arg: &'a mut T) -> preview1::ContextVFS<'a> {
            let r = self.0(arg);
            preview1::ContextVFS {
                vfs: &mut r.vfs,
                context: &mut r.preview1,
            }
        }
    }

    impl<T, F> genlayer_sdk::AddToLinkerFn<T> for Fwd<F>
    where
        F: Fn(&mut T) -> &mut Context + Copy + Send + Sync + 'static,
    {
        fn call<'a>(&self, arg: &'a mut T) -> genlayer_sdk::ContextVFS<'a> {
            let r = self.0(arg);
            genlayer_sdk::ContextVFS {
                vfs: &mut r.vfs,
                context: &mut r.genlayer_sdk,
            }
        }
    }

    preview1::add_to_linker_sync(linker, Fwd(f))?;
    genlayer_sdk::add_to_linker_sync(linker, Fwd(f))?;

    Ok(())
}
