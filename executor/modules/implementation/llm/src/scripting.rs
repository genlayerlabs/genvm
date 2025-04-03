use std::{collections::BTreeMap, sync::Arc};

use anyhow::Context;
use genvm_modules_impl_common::ModuleResult;
use genvm_modules_interfaces::llm as llm_iface;
use mlua::{IntoLua, LuaSerdeExt, UserDataRef};
use serde::{Deserialize, Serialize};

use crate::{config, handler};

pub struct UserVM {
    vm: mlua::Lua,
    exec_prompt: mlua::Function,
}

#[derive(Serialize)]
struct BackendInfo {
    pub models: Vec<String>,
}

#[derive(Serialize)]
struct Common {
    available_backends: BTreeMap<String, BackendInfo>,
}

impl mlua::UserData for handler::Handler {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        #[derive(Deserialize)]
        struct Args {
            provider: String,
            text: String,
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
                .config
                .backends
                .get(&args.provider)
                .ok_or(mlua::Error::DeserializeError("wrong provider".into()))?;
            let res = zelf
                .exec_prompt_in_provider(&args.text, llm_iface::OutputFormat::Text, provider_config)
                .await
                .with_context(|| "running in provider")?;
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

        let common = Common {
            available_backends: config
                .backends
                .iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        BackendInfo {
                            models: Vec::from([v.model.clone()]),
                        },
                    )
                })
                .collect(),
        };

        vm.globals().set("greyboxing", vm.to_value(&common)?)?;

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
