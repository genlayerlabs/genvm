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

    metrics: sync::DArc<super::Metrics>,
}

struct Handler(Arc<Inner>);

impl common::MessageHandler<web_iface::Message, FullResponse> for Handler {
    async fn handle(&self, message: web_iface::Message) -> common::ModuleResult<FullResponse> {
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

                Ok(FullResponse::Answer(RenderAnswer::Response(res)))
            }
            web_iface::Message::Render(payload, size_limit) => {
                let vm = &self.0.user_vm.vm;

                let payload_lua = vm.create_table()?;
                payload_lua.set(
                    "mode",
                    vm.to_value_with(&payload.mode, scripting::DEFAULT_LUA_SER_OPTIONS)?,
                )?;
                payload_lua.set("url", payload.url)?;
                payload_lua.set("wait_after_loaded", payload.wait_after_loaded.as_secs_f64())?;
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

                Ok(FullResponse::Answer(res))
            }

            web_iface::Message::GetStats => {
                let res = calldata::to_value(&self.0.metrics);
                Ok(FullResponse::GetStats(res))
            }
        }
    }

    async fn cleanup(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

#[derive(serde::Serialize)]
#[serde(untagged)]
pub enum FullResponse {
    Answer(web_iface::RenderAnswer),
    GetStats(calldata::Value),
}

impl<W: calldata::Writer> calldata::codec::Encode<W> for FullResponse {
    type Error = W::Error;

    fn encode(&self, enc: &mut calldata::Encoder<W>) -> std::result::Result<(), Self::Error> {
        match self {
            FullResponse::Answer(v) => calldata::codec::Encode::encode(v, enc),
            FullResponse::GetStats(v) => calldata::codec::Encode::encode(v, enc),
        }
    }
}

pub struct HandlerProvider {
    pub vm_pool: scripting::pool::Pool<ctx::VMData, Arc<ctx::CtxPart>, WebSubContext>,
    pub config: sync::DArc<super::config::Config>,
}

impl common::MessageHandlerProvider<genvm_modules_interfaces::web::Message, FullResponse>
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
        )?;
        Ok(sync::DArc::new(WebSubContext { scripting }))
    }

    async fn new_handler(
        &self,
        ctx: sync::DArc<WebSubContext>,
    ) -> anyhow::Result<
        impl common::MessageHandler<genvm_modules_interfaces::web::Message, FullResponse>,
    > {
        let user_vm = self.vm_pool.get();

        let (handler_ctx, ctx_val) = user_vm.create_ctx(&ctx)?;

        Ok(Handler(Arc::new(Inner {
            user_vm,
            _ctx: handler_ctx,
            ctx_val,
            metrics: sync::DArc::new(super::Metrics::default()),
        })))
    }
}
