use super::{ctx, prompt, providers, scripting, UserVM};
use crate::common::{MessageHandler, MessageHandlerProvider, ModuleError, ModuleResult};

use genvm_modules_interfaces::llm::{self as llm_iface};
use mlua::LuaSerdeExt;

use std::{collections::BTreeMap, sync::Arc};

pub struct Inner {
    user_vm: Arc<UserVM>,

    ctx: scripting::RSContext<ctx::CtxPart>,
    ctx_val: mlua::Value,
}

pub struct Provider {
    pub providers: Arc<BTreeMap<String, Box<dyn providers::Provider + Send + Sync>>>,
    pub vm_pool: scripting::pool::Pool<ctx::VMData, ctx::CtxPart>,
}

impl
    MessageHandlerProvider<
        genvm_modules_interfaces::llm::Message,
        genvm_modules_interfaces::llm::PromptAnswer,
    > for Provider
{
    async fn new_handler(
        &self,
        hello: genvm_modules_interfaces::GenVMHello,
    ) -> anyhow::Result<
        impl MessageHandler<
            genvm_modules_interfaces::llm::Message,
            genvm_modules_interfaces::llm::PromptAnswer,
        >,
    > {
        let client = reqwest::Client::new();

        let ctx = scripting::RSContext {
            client: client.clone(),
            data: Arc::new(ctx::CtxPart {
                hello,
                providers: self.providers.clone(),
                client,
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

struct Handler(Arc<Inner>);

impl crate::common::MessageHandler<llm_iface::Message, llm_iface::PromptAnswer> for Handler {
    async fn handle(
        &self,
        message: llm_iface::Message,
    ) -> crate::common::ModuleResult<llm_iface::PromptAnswer> {
        match message {
            llm_iface::Message::Prompt(payload) => {
                for img in &payload.images {
                    if let None = prompt::ImageType::sniff(&img.0) {
                        return Err(ModuleError {
                            causes: vec!["INVALID_IMAGE".into()],
                            fatal: false,
                            ctx: BTreeMap::new(),
                        }
                        .into());
                    }
                }
                self.0.exec_prompt(self.0.clone(), payload).await
            }
            llm_iface::Message::PromptTemplate(payload) => {
                self.0.exec_prompt_template(self.0.clone(), payload).await
            }
        }
    }

    async fn cleanup(&self) -> anyhow::Result<()> {
        Ok(())
    }
}

impl Inner {
    async fn exec_prompt(
        &self,
        _zelf: Arc<Inner>,
        payload: llm_iface::PromptPayload,
    ) -> ModuleResult<llm_iface::PromptAnswer> {
        log::debug!(payload:serde = payload, cookie = self.ctx.data.hello.cookie; "exec_prompt start");

        let payload = self.user_vm.vm.to_value(&payload)?;

        let res: mlua::Value = self
            .user_vm
            .call_fn(
                &self.user_vm.data.exec_prompt,
                (self.ctx_val.clone(), payload),
            )
            .await?;
        let res = self.user_vm.vm.from_value(res)?;

        log::debug!(result:serde = res, cookie = self.ctx.data.hello.cookie; "exec_prompt returned");

        Ok(res)
    }

    async fn exec_prompt_template(
        &self,
        _zelf: Arc<Inner>,
        payload: llm_iface::PromptTemplatePayload,
    ) -> ModuleResult<llm_iface::PromptAnswer> {
        log::debug!(payload:serde = payload, cookie = self.ctx.data.hello.cookie; "exec_prompt_template start");

        let payload = self.user_vm.vm.to_value(&payload)?;

        let res: mlua::Value = self
            .user_vm
            .call_fn(
                &self.user_vm.data.exec_prompt_template,
                (self.ctx_val.clone(), payload),
            )
            .await?;
        let res = self.user_vm.vm.from_value(res)?;

        log::debug!(result:serde = res, cookie = self.ctx.data.hello.cookie; "exec_prompt_template returned");

        Ok(res)
    }
}
