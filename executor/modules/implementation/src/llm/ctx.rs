use genvm_modules_interfaces::llm::{self as llm_iface};
use std::{collections::BTreeMap, sync::Arc};

use anyhow::Context;
use mlua::LuaSerdeExt;
use serde::Deserialize;

use crate::{common::ModuleResult, scripting};

use super::{config::Config, prompt, providers};

pub struct VMData {
    pub exec_prompt: mlua::Function,
    pub exec_prompt_template: mlua::Function,
}

pub struct CtxPart {
    pub hello: genvm_modules_interfaces::GenVMHello,
    pub providers: Arc<BTreeMap<String, Box<dyn providers::Provider + Send + Sync>>>,
    pub client: reqwest::Client,
}

impl mlua::UserData for CtxPart {}

impl CtxPart {
    pub async fn exec_prompt_in_provider(
        &self,
        prompt: &prompt::Internal,
        model: &str,
        provider_id: &str,
        format: prompt::ExtendedOutputFormat,
    ) -> ModuleResult<llm_iface::PromptAnswer> {
        log::debug!(
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
                .exec_prompt_text(&self.client, prompt, model)
                .await
                .map(llm_iface::PromptAnswer::Text),
            prompt::ExtendedOutputFormat::JSON => provider
                .exec_prompt_json(&self.client, prompt, model)
                .await
                .map(llm_iface::PromptAnswer::Object),
            prompt::ExtendedOutputFormat::Bool => provider
                .exec_prompt_bool_reason(&self.client, prompt, model)
                .await
                .map(llm_iface::PromptAnswer::Bool),
        };

        res.inspect_err(|err| {
            log::error!(
                prompt:serde = prompt,
                model = model,
                mode:? = format,
                provider_id = provider_id,
                error = genvm_common::log_error(err),
                cookie = self.hello.cookie;
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
    let (zelf, args) = args;
    let zelf: mlua::UserDataRef<Arc<CtxPart>> = zelf.get("__ctx_llm")?;

    let args: Args = vm
        .from_value(args)
        .with_context(|| "deserializing arguments")
        .map_err(scripting::anyhow_to_lua_error)?;

    let res = zelf
        .exec_prompt_in_provider(&args.prompt, &args.model, &args.provider, args.format)
        .await
        .with_context(|| "running in provider")
        .map_err(scripting::anyhow_to_lua_error)?;

    vm.to_value_with(&res, scripting::DEFAULT_LUA_SER_OPTIONS)
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
