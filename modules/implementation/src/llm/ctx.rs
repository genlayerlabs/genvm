use genvm_modules_interfaces::llm::{self as llm_iface};
use std::{collections::BTreeMap, sync::Arc};

use anyhow::Context as _;
use genvm_common::*;
use mlua::LuaSerdeExt;
use serde::Deserialize;

use crate::{common::ModuleResult, scripting};

use super::{config::Config, prompt, providers};

pub struct VMData {
    pub exec_prompt: mlua::Function,
    pub exec_prompt_template: mlua::Function,
    pub setup: Option<mlua::Function>,
    pub teardown: Option<mlua::Function>,
}

pub struct CtxPart {
    pub providers: Arc<BTreeMap<String, Box<dyn providers::Provider + Send + Sync>>>,
    pub metrics: sync::DArc<super::Metrics>,
}

impl mlua::UserData for CtxPart {}

impl CtxPart {
    pub async fn exec_prompt_in_provider(
        &self,
        dflt: &scripting::CtxPart,
        prompt: &prompt::Internal,
        model: &str,
        provider_id: &str,
        format: prompt::ExtendedOutputFormat,
    ) -> ModuleResult<providers::ProviderResponse<llm_iface::PromptAnswerData>> {
        log_debug!(
            prompt:serde = prompt,
            provider_id = provider_id,
            model = model,
            format:serde = format;
            "exec_prompt_in_provider"
        );

        let provider = self
            .providers
            .get(provider_id)
            .ok_or_else(|| anyhow::anyhow!("absent provider_id `{provider_id}`"))?;

        let res = match format {
            prompt::ExtendedOutputFormat::Text => provider
                .exec_prompt_text(dflt, prompt, model)
                .await
                .map(|resp| resp.map(llm_iface::PromptAnswerData::Text)),
            prompt::ExtendedOutputFormat::JSON => provider
                .exec_prompt_json_as_text(dflt, prompt, model)
                .await
                .map(|resp| resp.map(llm_iface::PromptAnswerData::Text)),
            prompt::ExtendedOutputFormat::Bool => provider
                .exec_prompt_bool_reason(dflt, prompt, model)
                .await
                .map(|resp| resp.map(llm_iface::PromptAnswerData::Bool)),
        };

        if let Ok(ref res) = res {
            let key = format!("{provider_id}/{model}");
            self.metrics
                .tokens
                .record(key, res.tokens.input, res.tokens.output, res.tokens.total);
        }

        res.inspect_err(|err| {
            log_info!(
                prompt:serde = prompt,
                model = model,
                mode:? = format,
                provider_id = provider_id,
                error:ah = err,
                genvm_id = dflt.hello.genvm_id;
                "prompt execution error (note: script should handle this error later)"
            );
        })
    }
}

/// NOTE: when changing fields, also update exec_prompt_in_provider in modules/install/lib/genvm-lua/lib-llm.lua
#[derive(Deserialize)]
struct Args {
    provider: String,
    prompt: prompt::Internal,
    format: prompt::ExtendedOutputFormat,
    model: String,
    #[serde(default)]
    timeout: Option<crate::common::Timeout>,
}

async fn exec_prompt_in_provider(
    vm: mlua::Lua,
    args: (mlua::Table, mlua::Value),
) -> Result<mlua::Value, mlua::Error> {
    let (table, args) = args;
    let ctx: mlua::UserDataRef<scripting::LuaDArc<CtxPart>> = table.get("__ctx_llm")?;
    let dflt: mlua::UserDataRef<scripting::LuaDArc<scripting::CtxPart>> =
        table.get("__ctx_dflt")?;

    let args: Args = vm
        .from_value(args)
        .with_context(|| "deserializing arguments")
        .map_err(scripting::anyhow_to_lua_error)?;

    let mut prompt = args.prompt;
    if args.timeout.is_some() {
        prompt.timeout = args.timeout;
    }

    let res = ctx
        .exec_prompt_in_provider(&dflt, &prompt, &args.model, &args.provider, args.format)
        .await
        .with_context(|| "running in provider")
        .map_err(scripting::anyhow_to_lua_error)?;

    let answer_data = vm.to_value_with(&res.result, scripting::DEFAULT_LUA_SER_OPTIONS)?;

    let tokens = vm.create_table()?;
    tokens.set("input", res.tokens.input)?;
    tokens.set("output", res.tokens.output)?;
    tokens.set("total", res.tokens.total)?;
    tokens.set("cache_read", res.tokens.cache_read_tokens)?;
    tokens.set("cache_write", res.tokens.cache_write_tokens)?;
    tokens.set("image_units", res.tokens.image_units)?;
    tokens.set(
        "raw_usage",
        vm.to_value_with(&res.tokens.raw_usage, scripting::DEFAULT_LUA_SER_OPTIONS)?,
    )?;

    let answer = vm.create_table()?;
    answer.set("data", answer_data)?;
    answer.set(
        "consumed_gen",
        scripting::rat::LuaRat(num_rational::BigRational::from(num_bigint::BigInt::from(0))),
    )?;
    answer.set("tokens", tokens)?;

    Ok(mlua::Value::Table(answer))
}

pub fn create_global(vm: &mlua::Lua, config: &Config) -> anyhow::Result<mlua::Value> {
    let llm = vm.create_table()?;
    llm.set(
        "exec_prompt_in_provider",
        vm.create_async_function(exec_prompt_in_provider)?,
    )?;

    let all_providers =
        BTreeMap::from_iter(config.backends.iter().map(|(k, v)| (k, &v.script_config)));
    llm.set(
        "providers",
        vm.to_value_with(&all_providers, scripting::DEFAULT_LUA_SER_OPTIONS)?,
    )?;

    llm.set(
        "templates",
        vm.to_value_with(&config.prompt_templates, scripting::DEFAULT_LUA_SER_OPTIONS)?,
    )?;

    llm.set(
        "meta",
        vm.to_value_with(&config.meta, scripting::DEFAULT_LUA_SER_OPTIONS)?,
    )?;

    llm.set(
        "timeout",
        vm.to_value_with(&config.timeout, scripting::DEFAULT_LUA_SER_OPTIONS)?,
    )?;

    llm.set(
        "exhaust",
        vm.create_function(|_vm: &mlua::Lua, _: ()| -> mlua::Result<()> {
            Err(mlua::Error::external(crate::common::BudgetExhausted))
        })?,
    )?;

    Ok(mlua::Value::Table(llm))
}
