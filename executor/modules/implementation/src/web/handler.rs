use super::{config, ctx};
use crate::{common, scripting};

use genvm_common::*;

use genvm_modules_interfaces::web::{self as web_iface, RenderAnswer};
use mlua::LuaSerdeExt;
use std::sync::Arc;

type UserVM = scripting::UserVM<ctx::VMData, ctx::CtxPart>;

pub struct Inner {
    user_vm: Arc<UserVM>,

    ctx: scripting::RSContext<ctx::CtxPart>,
    ctx_val: mlua::Value,
}

struct Handler(Arc<Inner>);

impl common::MessageHandler<web_iface::Message, web_iface::RenderAnswer> for Handler {
    async fn handle(
        &self,
        message: web_iface::Message,
    ) -> common::ModuleResult<web_iface::RenderAnswer> {
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

                Ok(RenderAnswer::Response(res))
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

                Ok(res)
            }
        }
    }

    async fn cleanup(&self) -> anyhow::Result<()> {
        let lock = self.0.ctx.data.session.lock().await;

        let session = match lock.as_ref() {
            None => return Ok(()),
            Some(session) => session,
        };

        if let Err(err) = self
            .0
            .ctx
            .client
            .delete(format!(
                "{}/session/{}",
                self.0.ctx.data.config.webdriver_host, session
            ))
            .send()
            .await
        {
            log_error!(error:err = err, id = session, cookie = self.0.ctx.data.hello.cookie; "session closed");
        } else {
            log_debug!(id = session, cookie = self.0.ctx.data.hello.cookie; "session closed");
        }
        Ok(())
    }
}

pub struct HandlerProvider {
    pub config: Arc<config::Config>,
    pub vm_pool: scripting::pool::Pool<ctx::VMData, ctx::CtxPart>,
}

impl
    common::MessageHandlerProvider<
        genvm_modules_interfaces::web::Message,
        genvm_modules_interfaces::web::RenderAnswer,
    > for HandlerProvider
{
    async fn new_handler(
        &self,
        hello: genvm_modules_interfaces::GenVMHello,
    ) -> anyhow::Result<
        impl common::MessageHandler<
            genvm_modules_interfaces::web::Message,
            genvm_modules_interfaces::web::RenderAnswer,
        >,
    > {
        let client = reqwest::Client::new();

        let ctx = scripting::RSContext {
            client: client.clone(),
            data: Arc::new(ctx::CtxPart {
                client,
                hello,
                session: tokio::sync::Mutex::new(None),
                config: self.config.clone(),
            }),
        };

        let user_vm = self.vm_pool.get();

        let ctx_val = user_vm.create_ctx(&ctx)?;

        Ok(Handler(Arc::new(Inner {
            user_vm,
            ctx,
            ctx_val,
        })))
    }
}
