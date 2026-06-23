use super::{ctx, prompt, scripting};
use crate::common::{MessageHandler, MessageHandlerProvider, ModuleError, ModuleResult};
use anyhow::Context as _;
use genvm_common::*;

use crate::common::LoggerWithId;
use genvm_modules_interfaces::llm::{self as llm_iface};
use mlua::LuaSerdeExt;

use std::{collections::BTreeMap, sync::Arc};

type VmGuard = scripting::pool::PoolGuard<ctx::VMData, sync::DArc<ctx::CtxPart>, LlmSubContext>;

pub struct Inner {
    user_vm: VmGuard,

    _ctx: sync::DArc<ctx::CtxPart>,
    ctx_val: mlua::Value,

    genvm_id: genvm_modules_interfaces::GenVMId,
}

type LlmSubContext = crate::manager::execution_context::LlmSubContext;

pub struct Provider {
    pub vm_pool: scripting::pool::Pool<ctx::VMData, sync::DArc<ctx::CtxPart>, LlmSubContext>,
    pub config: sync::DArc<super::config::Config>,
    pub providers: Arc<BTreeMap<String, Box<dyn super::providers::Provider + Send + Sync>>>,
}

impl MessageHandlerProvider<genvm_modules_interfaces::llm::Message, llm_iface::PromptAnswer>
    for Provider
{
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
            false,
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
    ) -> anyhow::Result<
        impl MessageHandler<genvm_modules_interfaces::llm::Message, llm_iface::PromptAnswer>,
    > {
        let genvm_id = ctx.scripting.hello.genvm_id;
        let user_vm = self.vm_pool.get().await;

        let (handler_ctx, ctx_val) = user_vm.create_ctx(&ctx)?;

        if let Some(ref setup) = user_vm.data.setup {
            let _: mlua::Value = user_vm
                .call_fn(setup, ctx_val.clone())
                .await
                .context("calling Setup")?;
        }

        Ok(Handler(Arc::new(Inner {
            user_vm,
            _ctx: handler_ctx,
            ctx_val,
            genvm_id,
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
            }
            llm_iface::Message::PromptTemplate {
                payload,
                remaining_fuel_as_gen,
            } => {
                self.0
                    .exec_prompt_template(self.0.clone(), payload, remaining_fuel_as_gen)
                    .await
            }
        }
    }

    async fn cleanup(&self) -> anyhow::Result<()> {
        if let Some(ref teardown) = self.0.user_vm.data.teardown {
            let _: mlua::Value = self
                .0
                .user_vm
                .call_fn(teardown, self.0.ctx_val.clone())
                .await
                .context("calling Teardown")?;
        }
        Ok(())
    }
}

impl Inner {
    fn u256_to_lua_rat(&self, val: primitive_types::U256) -> anyhow::Result<mlua::Value> {
        let buf = val.to_big_endian();
        let big = num_bigint::BigInt::from_bytes_be(num_bigint::Sign::Plus, &buf);
        let rat = num_rational::BigRational::from(big);
        Ok(mlua::Value::UserData(
            self.user_vm
                .vm
                .create_userdata(scripting::rat::LuaRat(rat))?,
        ))
    }

    fn lua_result_to_prompt_answer(
        &self,
        res: mlua::Value,
    ) -> anyhow::Result<llm_iface::PromptAnswer> {
        let table = res
            .as_table()
            .ok_or_else(|| anyhow::anyhow!("expected table from prompt result"))?;

        let consumed_gen = scripting::rat::lua_rat_to_u256(table)?;

        table.set("consumed_gen", mlua::Value::Nil)?;

        let data: llm_iface::PromptAnswerData = self
            .user_vm
            .vm
            .from_value(table.get::<mlua::Value>("data")?)?;

        Ok(llm_iface::PromptAnswer { data, consumed_gen })
    }

    fn try_catch_budget_exhausted(err: anyhow::Error) -> ModuleResult<llm_iface::PromptAnswer> {
        if err
            .downcast_ref::<crate::common::BudgetExhausted>()
            .is_some()
        {
            return Ok(llm_iface::PromptAnswer {
                data: llm_iface::PromptAnswerData::Text(String::new()),
                consumed_gen: primitive_types::U256::MAX,
            });
        }
        if let Some(mlua_err) = err.downcast_ref::<mlua::Error>() {
            if let mlua::Error::ExternalError(ext) = mlua_err {
                if ext
                    .downcast_ref::<crate::common::BudgetExhausted>()
                    .is_some()
                {
                    return Ok(llm_iface::PromptAnswer {
                        data: llm_iface::PromptAnswerData::Text(String::new()),
                        consumed_gen: primitive_types::U256::MAX,
                    });
                }
            }
        }
        Err(err)
    }

    async fn exec_prompt(
        &self,
        _zelf: Arc<Inner>,
        payload: llm_iface::PromptPayload,
        remaining_fuel_as_gen: primitive_types::U256,
    ) -> ModuleResult<llm_iface::PromptAnswer> {
        log_debug_into!(&LoggerWithId, payload:serde = payload, genvm_id:id = self.genvm_id.0; "exec_prompt start");

        let payload = self
            .user_vm
            .vm
            .to_value_with(&payload, scripting::DEFAULT_LUA_SER_OPTIONS)?;
        let fuel = self.u256_to_lua_rat(remaining_fuel_as_gen)?;

        let res: Result<mlua::Value, _> = self
            .user_vm
            .call_fn(
                &self.user_vm.data.exec_prompt,
                (self.ctx_val.clone(), payload, fuel),
            )
            .await;

        let res = match res {
            Ok(val) => self.lua_result_to_prompt_answer(val)?,
            Err(err) => Self::try_catch_budget_exhausted(err)?,
        };

        log_debug_into!(&LoggerWithId, result:serde = res, genvm_id:id = self.genvm_id.0; "exec_prompt returned");

        Ok(res)
    }

    async fn exec_prompt_template(
        &self,
        _zelf: Arc<Inner>,
        payload: llm_iface::PromptTemplatePayload,
        remaining_fuel_as_gen: primitive_types::U256,
    ) -> ModuleResult<llm_iface::PromptAnswer> {
        log_debug_into!(&LoggerWithId, payload:serde = payload, genvm_id:id = self.genvm_id.0; "exec_prompt_template start");

        let payload = self
            .user_vm
            .vm
            .to_value_with(&payload, scripting::DEFAULT_LUA_SER_OPTIONS)?;
        let fuel = self.u256_to_lua_rat(remaining_fuel_as_gen)?;

        let res: Result<mlua::Value, _> = self
            .user_vm
            .call_fn(
                &self.user_vm.data.exec_prompt_template,
                (self.ctx_val.clone(), payload, fuel),
            )
            .await;

        let res = match res {
            Ok(val) => self.lua_result_to_prompt_answer(val)?,
            Err(err) => Self::try_catch_budget_exhausted(err)?,
        };

        log_debug_into!(&LoggerWithId, result:serde = res, genvm_id:id = self.genvm_id.0; "exec_prompt_template returned");

        Ok(res)
    }
}
