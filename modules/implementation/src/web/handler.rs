use super::ctx;
use crate::{common, scripting};

use genvm_common::*;

use genvm_modules_interfaces::web::{self as web_iface, RenderAnswer};
use mlua::LuaSerdeExt;
use std::sync::Arc;

type UserVM = scripting::UserVM<ctx::VMData, Arc<ctx::CtxPart>>;

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
            web_iface::Message::Request(payload) => {
                let vm = &self.0.user_vm.vm;

                let payload_lua = vm.to_value(&payload)?;

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
            web_iface::Message::Render(payload) => {
                let vm = &self.0.user_vm.vm;

                let payload_lua = vm.create_table()?;
                payload_lua.set("mode", vm.to_value(&payload.mode)?)?;
                payload_lua.set("url", payload.url)?;
                payload_lua.set(
                    "wait_after_loaded",
                    payload.wait_after_loaded.0.as_secs_f64(),
                )?;

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
                let res = match calldata::to_value(&self.0.metrics) {
                    Ok(stats) => stats,
                    Err(e) => {
                        log_error!(error:err = e; "Failed to serialize metrics");
                        calldata::Value::Null
                    }
                };

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

pub struct HandlerProvider {
    pub vm_pool: scripting::pool::Pool<ctx::VMData, Arc<ctx::CtxPart>>,
}

impl common::MessageHandlerProvider<genvm_modules_interfaces::web::Message, FullResponse>
    for HandlerProvider
{
    async fn new_handler(
        &self,
        hello: genvm_modules_interfaces::GenVMHello,
    ) -> anyhow::Result<
        impl common::MessageHandler<genvm_modules_interfaces::web::Message, FullResponse>,
    > {
        let hello = Arc::new(hello);

        let metrics = sync::DArc::new(super::Metrics::default());

        let user_vm = self.vm_pool.get();

        let (ctx, ctx_val) = user_vm.create_ctx(&hello)?;

        Ok(Handler(Arc::new(Inner {
            user_vm,
            _ctx: ctx,
            ctx_val,
            metrics,
        })))
    }
}
