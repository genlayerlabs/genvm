use super::{ctx, prompt, scripting, UserVM};
use crate::common::{MessageHandler, MessageHandlerProvider, ModuleError, ModuleResult};
use genvm_common::*;

use genvm_modules_interfaces::llm::{self as llm_iface};
use mlua::LuaSerdeExt;

use std::{collections::BTreeMap, sync::Arc};

pub struct Inner {
    user_vm: Arc<UserVM>,

    ctx: Arc<ctx::CtxPart>,
    ctx_val: mlua::Value,

    metrics: sync::DArc<super::Metrics>,
}

pub struct Provider {
    pub vm_pool: scripting::pool::Pool<ctx::VMData, Arc<ctx::CtxPart>>,
}

#[derive(serde::Serialize)]
#[serde(untagged)]
pub enum FullResponse {
    Answer(llm_iface::PromptAnswer),
    GetStats(calldata::Value),
}

impl MessageHandlerProvider<genvm_modules_interfaces::llm::Message, FullResponse> for Provider {
    async fn new_handler(
        &self,
        hello: genvm_modules_interfaces::GenVMHello,
    ) -> anyhow::Result<impl MessageHandler<genvm_modules_interfaces::llm::Message, FullResponse>>
    {
        let hello = Arc::new(hello);

        let user_vm = self.vm_pool.get();

        let (ctx, ctx_val) = user_vm.create_ctx(&hello)?;

        Ok(Handler(Arc::new(Inner {
            metrics: ctx.metrics.clone(),
            user_vm,
            ctx,
            ctx_val,
        })))
    }
}

struct Handler(Arc<Inner>);

impl crate::common::MessageHandler<llm_iface::Message, FullResponse> for Handler {
    async fn handle(
        &self,
        message: llm_iface::Message,
    ) -> crate::common::ModuleResult<FullResponse> {
        match message {
            llm_iface::Message::Prompt {
                payload,
                remaining_fuel_as_gen,
            } => {
                for img in &payload.images {
                    if prompt::ImageType::sniff(&img.0).is_none() {
                        return Err(ModuleError {
                            causes: vec!["INVALID_IMAGE".into()],
                            fatal: false,
                            ctx: BTreeMap::new(),
                        }
                        .into());
                    }
                }
                self.0
                    .exec_prompt(self.0.clone(), payload, remaining_fuel_as_gen)
                    .await
                    .map(FullResponse::Answer)
            }
            llm_iface::Message::PromptTemplate {
                payload,
                remaining_fuel_as_gen,
            } => self
                .0
                .exec_prompt_template(self.0.clone(), payload, remaining_fuel_as_gen)
                .await
                .map(FullResponse::Answer),

            llm_iface::Message::GetStats => {
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

impl Inner {
    async fn exec_prompt(
        &self,
        _zelf: Arc<Inner>,
        payload: llm_iface::PromptPayload,
        remaining_fuel_as_gen: u64,
    ) -> ModuleResult<llm_iface::PromptAnswer> {
        log_debug!(payload:serde = payload, remaining_fuel_as_gen = remaining_fuel_as_gen, cookie = self.ctx.dflt.hello.cookie; "exec_prompt start");

        let payload = self.user_vm.vm.to_value(&payload)?;
        let fuel = self.user_vm.vm.to_value(&remaining_fuel_as_gen)?;

        let res: mlua::Value = self
            .user_vm
            .call_fn(
                &self.user_vm.data.exec_prompt,
                (self.ctx_val.clone(), payload, fuel),
            )
            .await?;
        let res = self.user_vm.vm.from_value(res)?;

        log_debug!(result:serde = res, cookie = self.ctx.dflt.hello.cookie; "exec_prompt returned");

        Ok(res)
    }

    async fn exec_prompt_template(
        &self,
        _zelf: Arc<Inner>,
        payload: llm_iface::PromptTemplatePayload,
        remaining_fuel_as_gen: u64,
    ) -> ModuleResult<llm_iface::PromptAnswer> {
        log_debug!(payload:serde = payload, remaining_fuel_as_gen = remaining_fuel_as_gen, cookie = self.ctx.dflt.hello.cookie; "exec_prompt_template start");

        let payload = self.user_vm.vm.to_value(&payload)?;
        let fuel = self.user_vm.vm.to_value(&remaining_fuel_as_gen)?;

        let res: mlua::Value = self
            .user_vm
            .call_fn(
                &self.user_vm.data.exec_prompt_template,
                (self.ctx_val.clone(), payload, fuel),
            )
            .await?;
        let res = self.user_vm.vm.from_value(res)?;

        log_debug!(result:serde = res, cookie = self.ctx.dflt.hello.cookie; "exec_prompt_template returned");

        Ok(res)
    }
}
