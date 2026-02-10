use genvm_modules_interfaces::llm::{self as llm_iface};
use std::{collections::BTreeMap, sync::Arc};

use anyhow::Context;
use genvm_common::*;
use mlua::LuaSerdeExt;
use serde::Deserialize;

use crate::{common::ModuleResult, scripting};

use super::{config::Config, prompt, providers};

pub struct VMData {
    pub exec_prompt: mlua::Function,
    pub exec_prompt_template: mlua::Function,
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
                .exec_prompt_json(dflt, prompt, model)
                .await
                .map(|resp| resp.map(llm_iface::PromptAnswerData::Object)),
            prompt::ExtendedOutputFormat::Bool => provider
                .exec_prompt_bool_reason(dflt, prompt, model)
                .await
                .map(|resp| resp.map(llm_iface::PromptAnswerData::Bool)),
        };

        res.inspect_err(|err| {
            log_error!(
                prompt:serde = prompt,
                model = model,
                mode:? = format,
                provider_id = provider_id,
                error:ah = err,
                genvm_id = dflt.hello.genvm_id;
                "prompt execution error"
            );
        })
    }
}

#[derive(Deserialize)]
struct Args {
    provider: String,
    prompt: prompt::Internal,
    format: prompt::ExtendedOutputFormat,
    model: String,
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

    let res = ctx
        .exec_prompt_in_provider(
            &dflt,
            &args.prompt,
            &args.model,
            &args.provider,
            args.format,
        )
        .await
        .with_context(|| "running in provider")
        .map_err(scripting::anyhow_to_lua_error)?;

    let answer = llm_iface::PromptAnswer {
        data: res.result,
        consumed_gen: 0,
    };

    let mlua::Value::Table(answer) =
        vm.to_value_with(&answer, scripting::DEFAULT_LUA_SER_OPTIONS)?
    else {
        std::unreachable!("to_value_with returned non-table for struct");
    };

    answer.set("input_tokens", res.tokens.input)?;
    answer.set("output_tokens", res.tokens.output)?;
    answer.set("total_tokens", res.tokens.total)?;

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
    llm.set("providers", vm.to_value(&all_providers)?)?;

    llm.set("templates", vm.to_value(&config.prompt_templates)?)?;

    Ok(mlua::Value::Table(llm))
}
