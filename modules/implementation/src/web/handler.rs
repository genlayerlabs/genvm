use super::ctx;
use crate::{common, scripting};

use genvm_common::*;

use genvm_modules_interfaces::web::{self as web_iface, RenderAnswer};
use mlua::LuaSerdeExt;
use std::sync::Arc;

type WebSubContext = crate::manager::execution_context::WebSubContext;
type UserVM = scripting::UserVM<ctx::VMData, Arc<ctx::CtxPart>, WebSubContext>;

pub struct Inner {
    user_vm: Arc<UserVM>,

    _ctx: Arc<ctx::CtxPart>,
    ctx_val: mlua::Value,

    max_wait_after_loaded: std::time::Duration,
}

struct Handler(Arc<Inner>);

impl common::MessageHandler<web_iface::Message, RenderAnswer> for Handler {
    async fn handle(&self, message: web_iface::Message) -> common::ModuleResult<RenderAnswer> {
        match message {
            web_iface::Message::Request(payload, size_limit) => {
                let vm = &self.0.user_vm.vm;

                let payload_lua = vm.to_value_with(&payload, scripting::DEFAULT_LUA_SER_OPTIONS)?;
                if let Some(table) = payload_lua.as_table() {
                    table.set("size_limit", size_limit)?;
                }

                let res: mlua::Value = self
                    .0
                    .user_vm
                    .call_fn(
                        &self.0.user_vm.data.request,
                        (self.0.ctx_val.clone(), payload_lua),
                    )
                    .await?;

                let res = self.0.user_vm.vm.from_value(res)?;

                Ok(RenderAnswer::Response(res))
            }
            web_iface::Message::Render(payload, size_limit) => {
                let vm = &self.0.user_vm.vm;

                let payload_lua = vm.create_table()?;
                payload_lua.set(
                    "mode",
                    vm.to_value_with(&payload.mode, scripting::DEFAULT_LUA_SER_OPTIONS)?,
                )?;
                payload_lua.set("url", payload.url)?;

                let max_wait = self.0.max_wait_after_loaded.as_secs_f64();
                let mut wait_after_loaded = payload.wait_after_loaded.as_secs_f64();
                if wait_after_loaded > max_wait {
                    log_warn!(
                        requested = wait_after_loaded,
                        max = max_wait;
                        "wait_after_loaded clamped to maximum"
                    );
                    wait_after_loaded = max_wait;
                }
                payload_lua.set("wait_after_loaded", wait_after_loaded)?;
                payload_lua.set("size_limit", size_limit)?;

                let res: mlua::Value = self
                    .0
                    .user_vm
                    .call_fn(
                        &self.0.user_vm.data.render,
                        (self.0.ctx_val.clone(), payload_lua),
                    )
                    .await?;

                let res = self.0.user_vm.vm.from_value(res)?;

                Ok(res)
            }
        }
    }

    async fn cleanup(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

pub struct HandlerProvider {
    pub vm_pool: scripting::pool::Pool<ctx::VMData, Arc<ctx::CtxPart>, WebSubContext>,
    pub config: sync::DArc<super::config::Config>,
}

impl common::MessageHandlerProvider<genvm_modules_interfaces::web::Message, RenderAnswer>
    for HandlerProvider
{
    type Ctx = WebSubContext;

    fn create_execution_context(
        &self,
        hello: genvm_modules_interfaces::GenVMHello,
    ) -> anyhow::Result<sync::DArc<WebSubContext>> {
        let hello = Arc::new(hello);
        let metrics = sync::DArc::new(super::Metrics::default());
        let scripting = crate::scripting::create_ctx_part(
            &hello,
            &self.config.gep(|x| &x.mod_base),
            metrics.gep(|x| &x.scripting),
            true,
        )?;
        Ok(sync::DArc::new(WebSubContext { scripting }))
    }

    async fn new_handler(
        &self,
        ctx: sync::DArc<WebSubContext>,
    ) -> anyhow::Result<
        impl common::MessageHandler<genvm_modules_interfaces::web::Message, RenderAnswer>,
    > {
        let user_vm = self.vm_pool.get();

        let (handler_ctx, ctx_val) = user_vm.create_ctx(&ctx)?;

        Ok(Handler(Arc::new(Inner {
            user_vm,
            _ctx: handler_ctx,
            ctx_val,
            max_wait_after_loaded: self.config.max_wait_after_loaded.to_duration(),
        })))
    }
}
