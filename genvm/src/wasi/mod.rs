use std::sync::Arc;

use crate::vm;

pub mod base;
pub(self) mod common;
pub mod genlayer_sdk;
pub mod preview1;

pub struct Context {
    pub preview1: preview1::Context,
    pub genlayer_sdk: genlayer_sdk::ContextData,
}

impl Context {
    pub fn new(
        data: genlayer_sdk::EssentialGenlayerSdkData,
        shared_data: Arc<vm::SharedData>,
    ) -> Self {
        Self {
            preview1: preview1::Context::new(),
            genlayer_sdk: genlayer_sdk::ContextData::new(data, shared_data),
        }
    }
}

pub(super) fn add_to_linker_sync<T: Send + 'static>(
    linker: &mut wasmtime::Linker<T>,
    f: impl Fn(&mut T) -> &mut Context + Copy + Send + Sync + 'static,
) -> anyhow::Result<()> {
    let fdup = f;
    preview1::add_to_linker_sync(linker, move |ctx| &mut fdup(ctx).preview1)?;
    genlayer_sdk::add_to_linker_sync(linker, move |ctx| &mut f(ctx).genlayer_sdk)?;
    Ok(())
}
