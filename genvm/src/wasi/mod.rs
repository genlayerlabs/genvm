use std::sync::Arc;

use crate::vm;

pub mod base;
pub(self) mod common;
pub mod genlayer_sdk;
pub mod preview1;

pub struct Context {
    vfs: common::VFS,
    pub preview1: preview1::Context,
    pub genlayer_sdk: genlayer_sdk::Context,
}

impl Context {
    pub fn new(
        data: genlayer_sdk::EssentialGenlayerSdkData,
        shared_data: Arc<vm::SharedData>,
    ) -> Self {
        Self {
            vfs: common::VFS::new(),
            preview1: preview1::Context::new(),
            genlayer_sdk: genlayer_sdk::Context::new(data, shared_data),
        }
    }
}

pub(super) fn add_to_linker_sync<T: Send + 'static>(
    linker: &mut wasmtime::Linker<T>,
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
