use std::{collections::BTreeMap, sync::Arc};

use anyhow::Context;
use genvm_modules_impl_common::ModuleResult;
use genvm_modules_interfaces::llm as llm_iface;
use mlua::{IntoLua, LuaSerdeExt, UserDataRef};
use serde::{Deserialize, Serialize};

use crate::{
    config,
    handler::{self, OverloadedError},
};

pub struct UserVM {
    vm: mlua::Lua,
    exec_prompt: mlua::Function,
}

#[derive(Serialize)]
struct Greyboxing {
    available_backends: BTreeMap<String, config::ScriptBackendConfig>,
}

impl mlua::UserData for handler::Handler {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        #[derive(Deserialize)]
        struct Args {
            provider: String,
            text: String,
            model: String,
        }

        async fn exec_in_backend(
            vm: mlua::Lua,
            args: (mlua::AnyUserData, mlua::Value),
        ) -> Result<mlua::Value, mlua::Error> {
            let (zelf, args) = args;
            let zelf: UserDataRef<Arc<handler::Handler>> =
                zelf.borrow().with_context(|| "unboxing userdata")?;
            let args: Args = vm
                .from_value(args)
                .with_context(|| "deserializing arguments")?;
            let provider_config = zelf
                .inner
                .config
                .backends
                .get(&args.provider)
                .ok_or(mlua::Error::DeserializeError("wrong provider".into()))?;
            let res = zelf
                .inner
                .exec_prompt_in_provider(
                    &args.text,
                    &args.model,
                    llm_iface::OutputFormat::Text,
                    provider_config,
                )
                .await
                .with_context(|| "running in provider");

            let res = match res {
                Ok(res) => res,
                Err(e) if e.is::<OverloadedError>() => {
                    return Err(mlua::Error::runtime("Overloaded"));
                }
                Err(e) => return Err(e.into()),
            };

            vm.to_value(&res)
        }
        methods.add_async_function("exec_in_backend", exec_in_backend);
    }
}

impl UserVM {
    pub fn new(config: &config::Config) -> anyhow::Result<Arc<UserVM>> {
        let vm = mlua::Lua::new();

        use mlua::StdLib;
        vm.load_std_libs(
            StdLib::COROUTINE
                | StdLib::TABLE
                | StdLib::IO
                | StdLib::STRING
                | StdLib::MATH
                | StdLib::PACKAGE,
        )?;

        let greyboxing = Greyboxing {
            available_backends: config
                .backends
                .iter()
                .map(|(k, v)| (k.clone(), v.script_config.clone()))
                .collect(),
        };

        let greyboxing = vm.to_value(&greyboxing)?;
        let log_fn = vm.create_function(|vm: &mlua::Lua, data: mlua::Value| {
            let as_serde: serde_json::Value = vm.from_value(data)?;
            log::info!(log:serde = as_serde; "script log");
            Ok(())
        })?;
        greyboxing.as_table().unwrap().set("log", log_fn)?;
        vm.globals().set("greyboxing", greyboxing)?;

        let user_script = std::fs::read_to_string(&config.lua_script_path)
            .with_context(|| format!("reading {}", config.lua_script_path))?;

        let chunk = vm.load(user_script);
        chunk.exec()?;

        let exec_prompt: mlua::Function = vm
            .globals()
            .get("exec_prompt")
            .with_context(|| "getting exec_prompt")?;

        log::info!("lua VM initialized");

        Ok(Arc::new(UserVM { vm, exec_prompt }))
    }

    pub async fn greybox(
        &self,
        handler: Arc<handler::Handler>,
        prompt: &str,
    ) -> ModuleResult<llm_iface::PromptAnswer> {
        let handler = self.vm.create_userdata(handler)?;
        let handler: mlua::Value = handler.into_lua(&self.vm)?;
        let prompt = self.vm.to_value(prompt)?;

        let arg = self
            .vm
            .create_table_from([("handler", handler), ("prompt", prompt)])?;

        let res: mlua::Value = self
            .exec_prompt
            .call_async(arg)
            .await
            .with_context(|| "calling user script")?;
        let res = self.vm.from_value(res)?;

        Ok(res)
    }
}
