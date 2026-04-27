use super::{ctx, prompt, scripting, UserVM};
use crate::common::{MessageHandler, MessageHandlerProvider, ModuleError, ModuleResult};
use genvm_common::*;

use crate::common::LoggerWithId;
use genvm_modules_interfaces::llm::{self as llm_iface};
use mlua::LuaSerdeExt;

use std::{collections::BTreeMap, sync::Arc};

pub struct Inner {
    user_vm: Arc<UserVM>,

    _ctx: sync::DArc<ctx::CtxPart>,
    ctx_val: mlua::Value,

    metrics: sync::DArc<super::Metrics>,
    genvm_id: genvm_modules_interfaces::GenVMId,
}

type LlmSubContext = crate::manager::execution_context::LlmSubContext;

pub struct Provider {
    pub vm_pool: scripting::pool::Pool<ctx::VMData, sync::DArc<ctx::CtxPart>, LlmSubContext>,
    pub config: sync::DArc<super::config::Config>,
    pub providers: Arc<BTreeMap<String, Box<dyn super::providers::Provider + Send + Sync>>>,
}

#[derive(serde::Serialize)]
#[serde(untagged)]
pub enum FullResponse {
    Answer(llm_iface::PromptAnswer),
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

impl MessageHandlerProvider<genvm_modules_interfaces::llm::Message, FullResponse> for Provider {
    type Ctx = LlmSubContext;

    fn create_execution_context(
        &self,
        hello: genvm_modules_interfaces::GenVMHello,
    ) -> anyhow::Result<sync::DArc<LlmSubContext>> {
        let hello = Arc::new(hello);
        let metrics = sync::DArc::new(super::Metrics::default());
        let scripting = crate::scripting::create_ctx_part(
            &hello,
            &self.config.gep(|x| &x.mod_base),
            metrics.gep(|x| &x.scripting),
        )?;
        let module = ctx::CtxPart {
            providers: self.providers.clone(),
            metrics,
        };
        Ok(sync::DArc::new(LlmSubContext { scripting, module }))
    }

    async fn new_handler(
        &self,
        ctx: sync::DArc<LlmSubContext>,
    ) -> anyhow::Result<impl MessageHandler<genvm_modules_interfaces::llm::Message, FullResponse>>
    {
        let genvm_id = ctx.scripting.hello.genvm_id;
        let user_vm = self.vm_pool.get();

        let (handler_ctx, ctx_val) = user_vm.create_ctx(&ctx)?;

        Ok(Handler(Arc::new(Inner {
            metrics: handler_ctx.metrics.clone(),
            user_vm,
            _ctx: handler_ctx,
            ctx_val,
            genvm_id,
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
                if payload.images.len() > 2 {
                    return Err(ModuleError {
                        causes: vec!["TOO_MANY_IMAGES".into()],
                        fatal: false,
                        ctx: BTreeMap::new(),
                    }
                    .into());
                }

                for img in &payload.images {
                    const IMG_MAX_SIZE: usize = 5 * 1024 * 1024; // 5 MB
                    if img.len() > IMG_MAX_SIZE {
                        return Err(ModuleError {
                            causes: vec!["IMAGE_TOO_LARGE".into()],
                            fatal: false,
                            ctx: BTreeMap::new(),
                        }
                        .into());
                    }

                    if prompt::ImageType::sniff(img.as_ref()).is_none() {
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
                let res = calldata::to_value(&self.0.metrics);
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
        mut payload: llm_iface::PromptPayload,
        remaining_fuel_as_gen: u64,
    ) -> ModuleResult<llm_iface::PromptAnswer> {
        log_debug_into!(&LoggerWithId, payload:serde = payload, remaining_fuel_as_gen = remaining_fuel_as_gen, genvm_id:id = self.genvm_id.0; "exec_prompt start");

        if payload.response_format == genvm_modules_interfaces::llm::OutputFormat::JSON2 {
            payload.response_format = genvm_modules_interfaces::llm::OutputFormat::JSON;
        }

        let payload = self
            .user_vm
            .vm
            .to_value_with(&payload, scripting::DEFAULT_LUA_SER_OPTIONS)?;
        let fuel = self
            .user_vm
            .vm
            .to_value_with(&remaining_fuel_as_gen, scripting::DEFAULT_LUA_SER_OPTIONS)?;

        let res: mlua::Value = self
            .user_vm
            .call_fn(
                &self.user_vm.data.exec_prompt,
                (self.ctx_val.clone(), payload, fuel),
            )
            .await?;
        let res = self.user_vm.vm.from_value(res)?;

        log_debug_into!(&LoggerWithId, result:serde = res, genvm_id:id = self.genvm_id.0; "exec_prompt returned");

        Ok(res)
    }

    async fn exec_prompt_template(
        &self,
        _zelf: Arc<Inner>,
        payload: llm_iface::PromptTemplatePayload,
        remaining_fuel_as_gen: u64,
    ) -> ModuleResult<llm_iface::PromptAnswer> {
        log_debug_into!(&LoggerWithId, payload:serde = payload, remaining_fuel_as_gen = remaining_fuel_as_gen, genvm_id:id = self.genvm_id.0; "exec_prompt_template start");

        let payload = self
            .user_vm
            .vm
            .to_value_with(&payload, scripting::DEFAULT_LUA_SER_OPTIONS)?;
        let fuel = self
            .user_vm
            .vm
            .to_value_with(&remaining_fuel_as_gen, scripting::DEFAULT_LUA_SER_OPTIONS)?;

        let res: mlua::Value = self
            .user_vm
            .call_fn(
                &self.user_vm.data.exec_prompt_template,
                (self.ctx_val.clone(), payload, fuel),
            )
            .await?;
        let res = self.user_vm.vm.from_value(res)?;

        log_debug_into!(&LoggerWithId, result:serde = res, genvm_id:id = self.genvm_id.0; "exec_prompt_template returned");

        Ok(res)
    }
}
