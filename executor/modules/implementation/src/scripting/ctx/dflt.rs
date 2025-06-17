use std::{collections::BTreeMap, sync::Arc};

use crate::{
    common::{self, ErrorKind, MapUserError, ModuleError},
    scripting::{self, DEFAULT_LUA_SER_OPTIONS},
};
use anyhow::Context;
use base64::Engine;
use genvm_common::*;
use genvm_modules_interfaces::GenericValue;
use mlua::LuaSerdeExt;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use genvm_modules_interfaces::web as web_iface;

fn default_none<T>() -> Option<T> {
    None
}

fn default_false() -> bool {
    false
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Request {
    pub method: web_iface::RequestMethod,
    pub url: String,
    pub headers: BTreeMap<String, web_iface::HeaderData>,

    #[serde(with = "serde_bytes", default = "default_none")]
    pub body: Option<Vec<u8>>,
    #[serde(default = "default_false")]
    pub sign: bool,
    #[serde(default = "default_false")]
    pub json: bool,
    #[serde(default = "default_false")]
    pub error_on_status: bool,
}

pub struct CtxPart {
    pub client: reqwest::Client,
}

impl mlua::UserData for CtxPart {}

impl CtxPart {
    async fn request(&self, vm: &mlua::Lua, req: Request) -> anyhow::Result<mlua::Value> {
        log_trace!(request:? = req; "received request");

        let url = match reqwest::Url::parse(&req.url) {
            Ok(url) => url,
            Err(err) => {
                return Err(common::ModuleError {
                    causes: vec![ErrorKind::DESERIALIZING.into()],
                    ctx: BTreeMap::from([
                        ("url".into(), GenericValue::Str(req.url)),
                        ("rust_error".into(), GenericValue::Str(format!("{err:#}"))),
                    ]),
                    fatal: true,
                }
                .into());
            }
        };

        let method = match req.method {
            web_iface::RequestMethod::GET => reqwest::Method::GET,
            web_iface::RequestMethod::POST => reqwest::Method::POST,
            web_iface::RequestMethod::DELETE => reqwest::Method::DELETE,
            web_iface::RequestMethod::HEAD => reqwest::Method::HEAD,
            web_iface::RequestMethod::OPTIONS => reqwest::Method::OPTIONS,
            web_iface::RequestMethod::PATCH => reqwest::Method::PATCH,
        };

        let mut headers: reqwest::header::HeaderMap<reqwest::header::HeaderValue> =
            reqwest::header::HeaderMap::with_capacity(req.headers.len());
        for (k, v) in req.headers.into_iter() {
            let name: reqwest::header::HeaderName = k
                .as_bytes()
                .try_into()
                .map_user_error(ErrorKind::DESERIALIZING.to_string(), true)?;
            let data: &[u8] = &v.0;
            headers.insert(
                name,
                data.try_into()
                    .map_user_error("invalid header value", true)?,
            );
        }

        let request = self.client.request(method, url).headers(headers);

        let request = if let Some(body) = req.body {
            request.body(body)
        } else {
            request
        };

        if req.json {
            let res = scripting::send_request_get_lua_compatible_response_json(
                &req.url,
                request,
                req.error_on_status,
            )
            .await?;
            Ok(vm.to_value_with(&res, DEFAULT_LUA_SER_OPTIONS)?)
        } else {
            let res = scripting::send_request_get_lua_compatible_response_bytes(
                &req.url,
                request,
                req.error_on_status,
            )
            .await?;
            Ok(vm.to_value_with(&res, DEFAULT_LUA_SER_OPTIONS)?)
        }
    }
}

pub fn create_global(vm: &mlua::Lua) -> anyhow::Result<mlua::Value> {
    let dflt = vm.create_table()?;

    dflt.set("log_json", vm.create_function(|vm: &mlua::Lua, data: mlua::Value| {
        let mut as_serde: BTreeMap<String, genvm_modules_interfaces::GenericValue> = vm.from_value(data)?;

        let level = as_serde.remove("level");
        let level = level.and_then(|x| x.as_str().map(|x| x.to_owned())).map(|x| logger::Level::from_str(&x).unwrap_or(logger::Level::Info)).unwrap_or(logger::Level::Info);

        let script_message = as_serde.remove("message").and_then(|x| x.as_str().map(|x| x.to_owned())).unwrap_or_else(|| "<none>".to_owned());

        log_with_level!(level, log:serde = as_serde, cookie = crate::common::get_cookie(); "script_log: {script_message}");
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
                return vm.to_value(&err);
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
                let zelf: mlua::UserDataRef<Arc<CtxPart>> = zelf
                    .borrow()
                    .with_context(|| "unboxing userdata")
                    .map_err(scripting::anyhow_to_lua_error)?;

                let request: Request = vm
                    .from_value(req)
                    .with_context(|| "deserializing request")
                    .map_err(scripting::anyhow_to_lua_error)?;

                let response = zelf
                    .request(&vm, request)
                    .await
                    .map_err(scripting::anyhow_to_lua_error)?;

                let result = vm.to_value_with(&response, DEFAULT_LUA_SER_OPTIONS)?;

                Ok(result)
            },
        )?,
    )?;

    Ok(mlua::Value::Table(dflt))
}

#[cfg(test)]
mod tests {
    use genvm_common::*;

    use crate::{
        common,
        scripting::{self, Response},
    };

    use super::*;

    async fn create_test_vm() -> scripting::UserVM<(), ()> {
        let mut cwd = std::env::current_dir().unwrap();
        cwd.push("scripting");
        let cwd = cwd.canonicalize().unwrap();
        let mut extra_path = cwd.to_str().unwrap().to_owned();
        extra_path.push_str("/?.lua");

        scripting::UserVM::create(&extra_path, |_| async { Ok(()) })
            .await
            .unwrap()
    }

    async fn test_status(status: u16) {
        common::tests::setup();

        let uvm = create_test_vm().await;

        let mut cwd = std::env::current_dir().unwrap();
        cwd.push("tests");
        cwd.push("lua");
        cwd.push("get_status.lua");
        let test_script = std::fs::read_to_string(cwd).unwrap();

        let chunk = uvm.vm.load(test_script);
        chunk.exec().unwrap();

        let f: mlua::Function = uvm.vm.globals().get("Test").unwrap();

        let rs_ctx = scripting::RSContext {
            client: common::create_client().unwrap(),
            data: Arc::new(()),
        };

        let ctx_lua = uvm.create_ctx(&rs_ctx).unwrap();

        let res: mlua::Value = f.call_async((ctx_lua, status.to_string())).await.unwrap();

        let res: Response = uvm.vm.from_value(res).unwrap();

        assert_eq!(res.status, status);
    }

    #[tokio::test]
    async fn test_status_200() {
        test_status(200).await;
    }

    #[tokio::test]
    async fn test_status_404() {
        test_status(404).await;
    }

    #[tokio::test]
    async fn test_echo_post() {
        common::tests::setup();

        let uvm = create_test_vm().await;

        let mut cwd = std::env::current_dir().unwrap();
        cwd.push("tests");
        cwd.push("lua");
        cwd.push("bytes.lua");
        let test_script = std::fs::read_to_string(cwd).unwrap();

        let chunk = uvm.vm.load(test_script);
        chunk.exec().unwrap();

        let f: mlua::Function = uvm.vm.globals().get("Test").unwrap();

        let expected = b"\xde\xad\xbe\xef";

        let rs_ctx = scripting::RSContext {
            client: common::create_client().unwrap(),
            data: Arc::new(()),
        };

        let ctx_lua = uvm.create_ctx(&rs_ctx).unwrap();

        let res: mlua::Value = f.call_async((ctx_lua,)).await.unwrap();

        let res: Response = uvm.vm.from_value(res).unwrap();

        log_trace!(response:? = res; "response");

        assert_eq!(res.status, 200);
        assert_eq!(res.body, expected);
    }
}
