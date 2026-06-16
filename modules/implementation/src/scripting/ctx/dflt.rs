use std::{collections::BTreeMap, sync::Arc};

use crate::{
    common::{ErrorKind, LoggerWithId, MapUserError, ModuleError},
    scripting::{self, DEFAULT_LUA_SER_OPTIONS},
};
use anyhow::Context as _;
use base64::Engine;
use genvm_common::*;
use mlua::LuaSerdeExt;
use std::str::FromStr;

use super::req::Request;

use super::CtxPart;

impl CtxPart {
    async fn resolve_host(&self, vm: &mlua::Lua, host: &str) -> anyhow::Result<mlua::Value> {
        let res = tokio::net::lookup_host(host).await?;
        let res = vm.create_sequence_from(res.map(|addr| addr.to_string()))?;
        Ok(mlua::Value::Table(res))
    }

    async fn request(&self, vm: &mlua::Lua, req: Request) -> anyhow::Result<mlua::Value> {
        log_trace!(request:serde = req; "received request");

        let is_json = req.json;
        let error_on_status = req.error_on_status;
        let url = req.url.as_str().to_owned();

        let body_size_limit = req.response_body_max_size.unwrap_or(usize::MAX);
        let request = req.into_reqwest(&self.client)?;

        if is_json {
            let res = scripting::send_request_get_lua_compatible_response_json(
                &self.metrics,
                &url,
                request,
                error_on_status,
                body_size_limit,
            )
            .await?;
            Ok(vm.to_value_with(&res, DEFAULT_LUA_SER_OPTIONS)?)
        } else {
            let res = scripting::send_request_get_lua_compatible_response_bytes(
                &self.metrics,
                &url,
                request,
                error_on_status,
                body_size_limit,
            )
            .await?;
            Ok(vm.to_value_with(&res, DEFAULT_LUA_SER_OPTIONS)?)
        }
    }
}

pub fn create_global(
    vm: &mlua::Lua,
    config: &crate::common::ModuleBaseConfig,
) -> anyhow::Result<mlua::Value> {
    let dflt = vm.create_table()?;

    dflt.set("data_dir", config.data_dir.as_str())?;

    dflt.set("log_json", vm.create_function(|vm: &mlua::Lua, data: mlua::Value| {
        let mut as_serde: BTreeMap<String, genvm_modules_interfaces::GenericValue> = vm.from_value(data)?;

        let level = as_serde.remove("level");
        let level = level.and_then(|x| x.as_str().map(|x| x.to_owned())).map(|x| logger::Level::from_str(&x).unwrap_or(logger::Level::Info)).unwrap_or(logger::Level::Info);

        let script_message = as_serde.remove("message").and_then(|x| x.as_str().map(|x| x.to_owned())).unwrap_or_else(|| "<none>".to_owned());

        log_with_level_into!(level, &LoggerWithId, log:serde = as_serde, genvm_id:id = crate::common::get_genvm_id().0; "script_log: {script_message}");
        Ok(())
    })?)?;

    dflt.set(
        "user_error",
        vm.create_function(|vm: &mlua::Lua, data: mlua::Value| {
            let as_serde: ModuleError = vm.from_value(data)?;

            Err::<(), mlua::Error>(mlua::Error::ExternalError(Arc::new(as_serde)))
        })?,
    )?;

    dflt.set(
        "sleep_seconds",
        vm.create_async_function(|vm: mlua::Lua, data: mlua::Value| async move {
            let as_seconds: f32 = vm.from_value(data)?;
            tokio::time::sleep(tokio::time::Duration::from_secs_f32(as_seconds)).await;

            Ok(())
        })?,
    )?;

    dflt.set(
        "base64_encode",
        vm.create_function(|vm: &mlua::Lua, data: mlua::String| {
            let encoded = base64::prelude::BASE64_STANDARD.encode(data.as_bytes());

            Ok(vm.create_string(encoded))
        })?,
    )?;

    dflt.set(
        "json_parse",
        vm.create_function(|vm: &mlua::Lua, data: mlua::String| {
            let data: serde_json::Value = serde_json::from_slice(&data.as_bytes())
                .map_user_error(ErrorKind::DESERIALIZING, true)
                .map_err(scripting::anyhow_to_lua_error)?;

            vm.to_value_with(&data, DEFAULT_LUA_SER_OPTIONS)
        })?,
    )?;

    dflt.set(
        "json_stringify",
        vm.create_function(|vm: &mlua::Lua, data: mlua::Value| {
            let data: serde_json::Value = vm.from_value(data)?;
            let data = serde_json::to_string(&data).map_err(mlua::Error::external)?;

            let res = vm.to_value_with(&data, DEFAULT_LUA_SER_OPTIONS)?;
            Ok(res)
        })?,
    )?;

    dflt.set(
        "base64_decode",
        vm.create_function(|vm: &mlua::Lua, data: mlua::String| {
            let decoded = base64::prelude::BASE64_STANDARD
                .decode(data.as_bytes())
                .map_user_error(ErrorKind::DESERIALIZING, true)
                .map_err(scripting::anyhow_to_lua_error)?;

            Ok(vm.create_string(decoded))
        })?,
    )?;

    dflt.set(
        "split_url",
        vm.create_function(
            |vm: &mlua::Lua, url: mlua::String| -> mlua::Result<mlua::Value> {
                let url_str = url.to_str()?;
                let url = match reqwest::Url::parse(&url_str) {
                    Ok(url) => url,
                    Err(_) => return Ok(mlua::Nil),
                };

                let ret = vm.create_table_from([
                    (
                        "schema",
                        mlua::Value::String(vm.create_string(url.scheme())?),
                    ),
                    (
                        "port",
                        if let Some(port) = url.port() {
                            mlua::Value::Number(port as f64)
                        } else {
                            mlua::Value::Nil
                        },
                    ),
                    (
                        "host",
                        mlua::Value::String(if let Some(host) = url.host_str() {
                            vm.create_string(host)?
                        } else {
                            vm.create_string(b"")?
                        }),
                    ),
                ])?;
                Ok(mlua::Value::Table(ret))
            },
        )?,
    )?;

    dflt.set(
        "resolve_host",
        vm.create_async_function(
            |vm: mlua::Lua, (ctx, host): (mlua::Table, String)| async move {
                let ctx: mlua::AnyUserData = ctx.get("__ctx_dflt")?;
                let ctx: mlua::UserDataRef<scripting::LuaDArc<CtxPart>> = ctx
                    .borrow()
                    .with_context(|| "unboxing userdata")
                    .map_err(scripting::anyhow_to_lua_error)?;

                ctx.resolve_host(&vm, &host)
                    .await
                    .map_err(scripting::anyhow_to_lua_error)
            },
        )?,
    )?;

    dflt.set(
        "set_host",
        vm.create_function(|vm: &mlua::Lua, (url, host): (String, String)| {
            let mut url = match reqwest::Url::parse(&url) {
                Ok(url) => url,
                Err(_) => {
                    return Err(scripting::anyhow_to_lua_error(anyhow::anyhow!(
                        "invalid url"
                    )))
                }
            };

            url.set_host(Some(&host))
                .map_err(|e| scripting::anyhow_to_lua_error(e.into()))?;
            Ok(vm.create_string(url.as_str())?)
        })?,
    )?;

    dflt.set(
        "url_encode",
        vm.create_function(|vm: &mlua::Lua, data: mlua::String| {
            let encoded =
                url::form_urlencoded::byte_serialize(&data.as_bytes()).collect::<String>();
            vm.create_string(encoded)
        })?,
    )?;

    dflt.set(
        "filter_text",
        vm.create_function(|vm: &mlua::Lua, data: (mlua::String, mlua::Value)| {
            let vals: Vec<super::filters::TextFilter> = vm.from_value(data.1)?;
            let res = super::filters::apply_filters(&data.0.to_str()?, &vals);
            vm.create_string(res)
        })?,
    )?;

    dflt.set(
        "filter_image",
        vm.create_function(|vm: &mlua::Lua, data: (mlua::String, mlua::Value)| {
            let vals: Vec<super::filters::ImageFilter> = vm.from_value(data.1)?;
            let res = super::filters::apply_image_filters(&data.0.as_bytes(), &vals)
                .map_err(scripting::anyhow_to_lua_error)?;
            vm.create_string(res)
        })?,
    )?;

    dflt.set(
        "as_user_error",
        vm.create_function(|vm: &mlua::Lua, args: mlua::Value| {
            log_trace!(name = args.type_name(); "casting to user error (1)");

            let err = match args.as_error() {
                None => return Ok(mlua::Value::Nil),
                Some(err) => err,
            };

            log_trace!(error:? = err; "casting to user error (2)");

            if let Some(err) = super::try_unwrap_err(err) {
                log_trace!(error:? = err; "casting to user error (3)");
                return vm.to_value_with(&err, DEFAULT_LUA_SER_OPTIONS);
            }

            Ok(mlua::Value::Nil)
        })?,
    )?;

    dflt.set(
        "request",
        vm.create_async_function(
            |vm: mlua::Lua, args: (mlua::Table, mlua::Value)| async move {
                let (zelf, req) = args;

                let zelf: mlua::AnyUserData = zelf.get("__ctx_dflt")?;
                let zelf: mlua::UserDataRef<scripting::LuaDArc<CtxPart>> = zelf
                    .borrow()
                    .with_context(|| "unboxing userdata")
                    .map_err(scripting::anyhow_to_lua_error)?;

                let mut request: Request = vm
                    .from_value(req)
                    .with_context(|| "deserializing request")
                    .map_err(scripting::anyhow_to_lua_error)?;

                if request.sign {
                    request
                        .add_rfc9421_sign_headers(&zelf)
                        .await
                        .map_err(mlua::Error::external)?;
                }

                let response = zelf
                    .request(&vm, request)
                    .await
                    .map_err(scripting::anyhow_to_lua_error)?;

                let result = vm.to_value_with(&response, DEFAULT_LUA_SER_OPTIONS)?;

                Ok(result)
            },
        )?,
    )?;

    use rand::TryRngCore;

    dflt.set(
        "random_bytes",
        vm.create_function(|vm: &mlua::Lua, length: usize| {
            let mut rng = rand::rngs::OsRng;
            let mut bytes = vec![0u8; length];
            rng.try_fill_bytes(&mut bytes)
                .map_err(|x| scripting::anyhow_to_lua_error(x.into()))?;
            vm.create_string(bytes)
        })?,
    )?;

    dflt.set(
        "random_float",
        vm.create_function(|_: &mlua::Lua, _: ()| {
            let mut rng = rand::rngs::OsRng;
            let float_size = 64;
            let precision = 52 + 1;
            let scale = 1.0 / ((1u64 << precision) as f64);

            let value: u64 = rng
                .try_next_u64()
                .map_err(|x| scripting::anyhow_to_lua_error(x.into()))?;
            let value = value >> (float_size - precision) as u64;

            Ok(scale * value as f64)
        })?,
    )?;

    Ok(mlua::Value::Table(dflt))
}
