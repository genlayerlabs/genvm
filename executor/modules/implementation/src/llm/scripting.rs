use std::{collections::BTreeMap, sync::Arc};

use crate::common::ModuleResult;
use anyhow::Context;
use genvm_modules_interfaces::llm as llm_iface;
use mlua::{IntoLua, LuaSerdeExt, UserDataRef};
use serde::{Deserialize, Serialize};

use super::{
    config,
    handler::{self, OverloadedError},
    prompt,
};

pub struct UserVM {
    vm: mlua::Lua,
    exec_prompt: mlua::Function,
    exec_prompt_template: mlua::Function,
}

#[derive(Serialize)]
struct Greyboxing {
    available_backends: BTreeMap<String, config::ScriptBackendConfig>,
    templates: serde_json::Value,
}

impl mlua::UserData for handler::Handler {
    fn add_methods<M: mlua::UserDataMethods<Self>>(methods: &mut M) {
        #[derive(Deserialize)]
        struct Args {
            provider: String,
            prompt: prompt::Internal,
            format: prompt::ExtendedOutputFormat,
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

            let res = zelf
                .exec_prompt_in_provider(&args.prompt, &args.model, &args.provider, args.format)
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
        use mlua::StdLib;

        let lua_lib_path = {
            let mut lua_lib_path = std::env::current_exe()?;
            lua_lib_path.pop();
            lua_lib_path.pop();
            lua_lib_path.push("share");
            lua_lib_path.push("lib");
            lua_lib_path.push("genvm");
            lua_lib_path.push("greyboxing");

            let mut path = lua_lib_path
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("could not detect default lib path"))?
                .to_owned();
            path.push_str("/?.lua");

            path
        };

        std::env::set_var("LUA_PATH", &lua_lib_path);

        let lua_libs = StdLib::COROUTINE
            | StdLib::TABLE
            | StdLib::IO
            | StdLib::STRING
            | StdLib::MATH
            | StdLib::PACKAGE;

        let vm = mlua::Lua::new_with(lua_libs, mlua::LuaOptions::default())?;

        vm.globals().set("LUA_PATH", lua_lib_path)?;

        vm.load_std_libs(lua_libs)?;

        let greyboxing = Greyboxing {
            available_backends: config
                .backends
                .iter()
                .map(|(k, v)| (k.clone(), v.script_config.clone()))
                .collect(),
            templates: serde_json::to_value(&config.prompt_templates)?,
        };

        let greyboxing = vm.to_value(&greyboxing)?;
        let log_fn = vm.create_function(|vm: &mlua::Lua, data: mlua::Value| {
            let as_serde: serde_json::Value = vm.from_value(data)?;
            log::info!(log:serde = as_serde, cookie = crate::common::get_cookie(); "script log");
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

        let exec_prompt_template: mlua::Function = vm
            .globals()
            .get("exec_prompt_template")
            .with_context(|| "getting exec_prompt_template")?;

        log::info!("lua VM initialized");

        Ok(Arc::new(UserVM {
            vm,
            exec_prompt,
            exec_prompt_template,
        }))
    }

    pub async fn greybox(
        &self,
        handler: Arc<handler::Handler>,
        payload: &llm_iface::PromptPayload,
    ) -> ModuleResult<llm_iface::PromptAnswer> {
        let host_data = self.vm.to_value(&handler.hello.host_data)?;

        let handler = self.vm.create_userdata(handler)?;
        let handler: mlua::Value = handler.into_lua(&self.vm)?;
        let payload = self.vm.to_value(payload)?;

        let arg = self.vm.create_table_from([
            ("handler", handler),
            ("payload", payload),
            ("host_data", host_data),
        ])?;

        let res: mlua::Value = self
            .exec_prompt
            .call_async(arg)
            .await
            .with_context(|| "calling user script")?;
        let res = self.vm.from_value(res)?;

        Ok(res)
    }

    pub async fn greybox_template(
        &self,
        handler: Arc<handler::Handler>,
        payload: llm_iface::PromptTemplatePayload,
    ) -> ModuleResult<llm_iface::PromptAnswer> {
        let host_data = self.vm.to_value(&handler.hello.host_data)?;

        let handler = self.vm.create_userdata(handler)?;
        let handler: mlua::Value = handler.into_lua(&self.vm)?;
        let payload = self.vm.to_value(&payload)?;

        let arg = self.vm.create_table_from([
            ("handler", handler),
            ("payload", payload),
            ("host_data", host_data),
        ])?;

        let res: mlua::Value = self
            .exec_prompt_template
            .call_async(arg)
            .await
            .with_context(|| "calling user script")?;
        let res = self.vm.from_value(res)?;

        Ok(res)
    }
}
