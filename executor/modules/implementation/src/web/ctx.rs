use std::sync::Arc;

use anyhow::Context;
use mlua::LuaSerdeExt;

use crate::{
    common::{self, MapUserError, ModuleResult},
    scripting::DEFAULT_LUA_SER_OPTIONS,
};

use super::{
    config::{self, Config},
    domains,
};

pub struct VMData {
    pub render: mlua::Function,
}

pub struct CtxPart {
    pub hello: genvm_modules_interfaces::GenVMHello,
    pub session: tokio::sync::Mutex<Option<String>>,
    pub client: reqwest::Client,
    pub config: Arc<config::Config>,
}

impl mlua::UserData for CtxPart {}

impl CtxPart {
    pub async fn get_webdriver_session(&self) -> ModuleResult<String> {
        if let Some(session) = self.session.lock().await.as_ref() {
            return Ok(session.clone());
        }

        let create_request = self
            .client
            .post(format!("{}/session", &self.config.webdriver_host))
            .header("Content-Type", "application/json; charset=utf-8")
            .body(self.config.session_create_request.clone());
        log::trace!(request:? = create_request, body = self.config.session_create_request, cookie = self.hello.cookie; "creating session");
        let opened_session_res = create_request
            .send()
            .await
            .with_context(|| "creating sessions request")?;
        let body = common::read_response(opened_session_res)
            .await
            .with_context(|| "reading response")?;
        let val: serde_json::Value = serde_json::from_str(&body)?;
        let session_id = val
            .pointer("/value/sessionId")
            .and_then(|val| val.as_str())
            .ok_or_else(|| anyhow::anyhow!("invalid json {}", val))?;

        let session_id = session_id.to_owned();

        let mut lock = self.session.lock().await;
        *lock = Some(session_id.clone());

        Ok(session_id)
    }
}

pub fn create_global(vm: &mlua::Lua, config: &Config) -> anyhow::Result<mlua::Value> {
    let web = vm.create_table()?;

    web.set("config", vm.to_value_with(config, DEFAULT_LUA_SER_OPTIONS)?)?;

    let tld = vm.create_table_from(domains::DOMAINS.iter().map(|k| (*k, true)))?;
    for k in &config.extra_tld {
        tld.set(&**k, true)?;
    }
    web.set("allowed_tld", tld)?;

    web.set(
        "get_webdriver_session",
        vm.create_async_function(|_vm: mlua::Lua, ctx: mlua::Table| async move {
            let web: mlua::UserDataRef<Arc<CtxPart>> = ctx.get("__ctx_web")?;

            let res = web
                .get_webdriver_session()
                .await
                .map_user_error("CREATING_SESSION", true)?;
            Ok(res)
        })?,
    )?;

    Ok(mlua::Value::Table(web))
}
