use std::{collections::BTreeMap, sync::Arc};

use crate::{
    common::{ErrorKind, MapUserError, ModuleError},
    scripting::{self, DEFAULT_LUA_SER_OPTIONS},
};
use anyhow::Context;
use base64::Engine;
use genvm_common::*;
use mlua::LuaSerdeExt;
use std::str::FromStr;

use super::req::Request;

pub struct CtxPart {
    pub client: reqwest::Client,
    pub sign_url: Arc<str>,
    pub sign_headers: Arc<BTreeMap<String, String>>,
    pub sign_vars: BTreeMap<String, String>,
    pub node_address: String,
    pub tx_id: String,
}

impl mlua::UserData for CtxPart {}

impl CtxPart {
    async fn request(&self, vm: &mlua::Lua, req: Request) -> anyhow::Result<mlua::Value> {
        log_trace!(request:? = req; "received request");

        let is_json = req.json;
        let error_on_status = req.error_on_status;
        let url = req.url.as_str().to_owned();

        let request = req.into_reqwest(&self.client)?;

        if is_json {
            let res = scripting::send_request_get_lua_compatible_response_json(
                &url,
                request,
                error_on_status,
            )
            .await?;
            Ok(vm.to_value_with(&res, DEFAULT_LUA_SER_OPTIONS)?)
        } else {
            let res = scripting::send_request_get_lua_compatible_response_bytes(
                &url,
                request,
                error_on_status,
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

        let conf = common::ModuleBaseConfig {
            bind_address: "".to_owned(),
            vm_count: 1,
            lua_script_path: "".to_owned(),
            extra_lua_path: extra_path,
            signer_headers: Arc::new(BTreeMap::new()),
            signer_url: Arc::from(""),
        };

        scripting::UserVM::create(&conf, |_| async { Ok(()) })
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
            hello: Arc::new(genvm_modules_interfaces::GenVMHello {
                cookie: "test_cookie".to_owned(),
                host_data: genvm_modules_interfaces::HostData {
                    node_address: "test_node_address".to_owned(),
                    tx_id: "test_tx_id".to_owned(),
                    rest: serde_json::Map::new(),
                },
            }),
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

        let hello = common::tests::get_hello();

        let rs_ctx = scripting::RSContext {
            client: common::create_client().unwrap(),
            data: Arc::new(()),
            hello,
        };

        let ctx_lua = uvm.create_ctx(&rs_ctx).unwrap();

        let res: mlua::Value = f.call_async((ctx_lua,)).await.unwrap();

        let res: Response = uvm.vm.from_value(res).unwrap();

        log_trace!(response:? = res; "response");

        assert_eq!(res.status, 200);
        assert_eq!(res.body, expected);
    }
}
