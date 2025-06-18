pub mod pool;

mod ctx;

use genvm_common::*;
use genvm_modules_interfaces::{web::HeaderData, GenericValue};
use mlua::LuaSerdeExt;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, future::Future, sync::Arc};

use crate::common::{self, MapUserError, ModuleError};

pub struct RSContext<C> {
    pub client: reqwest::Client,
    pub hello: Arc<genvm_modules_interfaces::GenVMHello>,
    pub data: Arc<C>,
}

pub type CtxCreator<C> =
    dyn Fn(&RSContext<C>, &mlua::Lua, &mlua::Table) -> anyhow::Result<()> + Send + Sync;

pub struct UserVM<T, C> {
    pub vm: mlua::Lua,
    pub data: T,

    ctx_creators: Vec<Box<CtxCreator<C>>>,
}

pub fn anyhow_to_lua_error(e: anyhow::Error) -> mlua::Error {
    match e.downcast::<mlua::Error>() {
        Ok(e) => e,
        Err(e) => match e.downcast::<ModuleError>() {
            Ok(e) => mlua::Error::external(e),
            Err(e) => {
                // we may need to *relocate* to allow other type checks
                mlua::Error::external(e.into_boxed_dyn_error())
            }
        },
    }
}

impl<T, C> UserVM<T, C> {
    pub fn add_ctx_creator(&mut self, creator: Box<CtxCreator<C>>) {
        self.ctx_creators.push(creator);
    }

    pub fn create_ctx(&self, rs_ctx: &RSContext<C>) -> anyhow::Result<mlua::Value> {
        let ctx = self.vm.create_table()?;

        for c in &self.ctx_creators {
            c(rs_ctx, &self.vm, &ctx)?;
        }

        Ok(mlua::Value::Table(ctx))
    }

    pub async fn create<F>(
        mod_config: &common::ModuleBaseConfig,
        data_getter: impl FnOnce(mlua::Lua) -> F,
    ) -> anyhow::Result<Self>
    where
        F: Future<Output = anyhow::Result<T>>,
    {
        use mlua::StdLib;

        let lua_lib_path = {
            let mut lua_lib_path = std::env::current_exe()?;
            lua_lib_path.pop();
            lua_lib_path.pop();
            lua_lib_path.push("share");
            lua_lib_path.push("lib");
            lua_lib_path.push("genvm");
            lua_lib_path.push("lua");

            let mut path = lua_lib_path
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("could not detect default lib path"))?
                .to_owned();
            path.push_str("/?.lua");

            if !mod_config.extra_lua_path.is_empty() {
                path.push(';');
                path.push_str(&mod_config.extra_lua_path);
            }

            log_info!(path = path; "lua path");

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

        vm.load_std_libs(lua_libs)?;

        vm.globals().set("__dflt", ctx::dflt::create_global(&vm)?)?;

        let mut ctx_creators: Vec<Box<CtxCreator<C>>> = Vec::new();

        let signer_headers = mod_config.signer_headers.clone();
        let sign_url = mod_config.signer_url.clone();

        ctx_creators.push(Box::new(move |rs_ctx, vm, ctx| {
            let mut sign_vars = BTreeMap::new();

            sign_vars.insert(
                "node_address".to_owned(),
                rs_ctx.hello.host_data.node_address.clone(),
            );
            sign_vars.insert("tx_id".to_owned(), rs_ctx.hello.host_data.tx_id.clone());
            for (k, v) in &rs_ctx.hello.host_data.rest {
                if let serde_json::Value::String(s) = v {
                    sign_vars.insert(k.clone(), s.clone());
                }
            }

            let my_ctx = vm.create_userdata(Arc::new(ctx::dflt::CtxPart {
                client: rs_ctx.client.clone(),
                node_address: rs_ctx.hello.host_data.node_address.clone(),
                tx_id: rs_ctx.hello.host_data.tx_id.clone(),
                sign_vars,
                sign_headers: signer_headers.clone(),
                sign_url: sign_url.clone(),
            }))?;

            ctx.set("__ctx_dflt", my_ctx)?;

            let hello_value = vm.to_value(&rs_ctx.hello)?;
            let hello_value = hello_value
                .as_table()
                .ok_or_else(|| mlua::Error::external("expected hello value to be a table"))?;

            for kv in hello_value.pairs() {
                let (k, v): (mlua::Value, mlua::Value) = kv?;
                ctx.set(k, v)?;
            }

            Ok(())
        }));

        Ok(Self {
            data: data_getter(vm.clone()).await?,
            ctx_creators,
            vm,
        })
    }

    pub async fn call_fn<R>(
        &self,
        f: &mlua::Function,
        args: impl mlua::IntoLuaMulti,
    ) -> anyhow::Result<R>
    where
        R: mlua::FromLuaMulti,
    {
        let res = f.call_async(args).await;

        match res {
            Ok(res) => Ok(res),
            Err(mlua::Error::ExternalError(e)) => Err(anyhow::Error::from(e)),
            Err(mlua::Error::WithContext { context, cause }) => {
                Err(anyhow::Error::from(cause).context(context))
            }
            Err(e) => Err(anyhow::Error::from(e)),
        }
    }
}

pub const DEFAULT_LUA_SER_OPTIONS: mlua::SerializeOptions = mlua::SerializeOptions::new()
    .serialize_none_to_null(false)
    .serialize_unit_to_null(false);

pub async fn load_script<P>(vm: &mlua::Lua, path: P) -> anyhow::Result<()>
where
    P: AsRef<std::path::Path> + Into<String>,
{
    let script_contents = std::fs::read_to_string(&path)?;
    let chunk = vm.load(script_contents);

    let mut name = String::from("@");
    name.push_str(&path.into());

    let chunk = chunk.set_name(name);
    chunk.exec_async().await?;

    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Response {
    pub status: u16,

    pub headers: BTreeMap<String, HeaderData>,

    #[serde(with = "serde_bytes")]
    pub body: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResponseJSON {
    pub status: u16,

    pub headers: BTreeMap<String, HeaderData>,

    pub body: serde_json::Value,
}

pub async fn send_request_get_lua_compatible_response_bytes(
    url: &str,
    request: reqwest::RequestBuilder,
    error_on_status: bool,
) -> anyhow::Result<Response> {
    let response = request
        .send()
        .await
        .map_user_error(common::ErrorKind::SENDING_REQUEST, true)?;

    let status = response.status().as_u16();
    let mut new_headers = BTreeMap::<String, HeaderData>::new();
    for (k, v) in response.headers() {
        new_headers.insert(k.as_str().to_owned(), HeaderData(v.as_bytes().to_owned()));
    }

    let body = response.bytes().await;

    let body = match body {
        Ok(body) => body,
        Err(e) => {
            return Err(ModuleError {
                causes: vec![common::ErrorKind::READING_BODY.into()],
                fatal: true,
                ctx: BTreeMap::from([
                    ("url".to_owned(), GenericValue::Str(url.to_owned())),
                    ("status".to_string(), GenericValue::Number(status.into())),
                    ("rust_error".to_owned(), GenericValue::Str(e.to_string())),
                    (
                        "headers".to_owned(),
                        GenericValue::Map(BTreeMap::from_iter(
                            new_headers
                                .into_iter()
                                .map(|(k, v)| (k, GenericValue::Bytes(v.0))),
                        )),
                    ),
                ]),
            }
            .into());
        }
    };

    log_trace!(body:? = body, len = body.len(); "read body");

    if error_on_status && status != 200 {
        return Err(ModuleError {
            causes: vec![common::ErrorKind::STATUS_NOT_OK.into()],
            fatal: true,
            ctx: BTreeMap::from([
                ("url".to_owned(), GenericValue::Str(url.to_owned())),
                ("status".to_string(), GenericValue::Number(status.into())),
                (
                    "headers".to_owned(),
                    GenericValue::Map(BTreeMap::from_iter(
                        new_headers
                            .into_iter()
                            .map(|(k, v)| (k, GenericValue::Bytes(v.0))),
                    )),
                ),
                ("body".to_owned(), GenericValue::Bytes(body.into())),
            ]),
        }
        .into());
    }

    Ok(Response {
        status,
        headers: new_headers,
        body: body.into(),
    })
}

pub async fn send_request_get_lua_compatible_response_json(
    url: &str,
    request: reqwest::RequestBuilder,
    error_on_status: bool,
) -> anyhow::Result<ResponseJSON> {
    let response = request
        .send()
        .await
        .map_user_error(common::ErrorKind::SENDING_REQUEST, true)?;

    let status = response.status().as_u16();
    let mut new_headers = BTreeMap::<String, HeaderData>::new();
    for (k, v) in response.headers() {
        new_headers.insert(k.as_str().to_owned(), HeaderData(v.as_bytes().to_owned()));
    }

    let body = response.json().await;

    let body: serde_json::Value = match body {
        Ok(body) => body,
        Err(e) => {
            return Err(ModuleError {
                causes: vec![common::ErrorKind::READING_BODY.into()],
                fatal: true,
                ctx: BTreeMap::from([
                    ("url".to_owned(), GenericValue::Str(url.to_owned())),
                    ("status".to_string(), GenericValue::Number(status.into())),
                    ("rust_error".to_owned(), GenericValue::Str(e.to_string())),
                    (
                        "headers".to_owned(),
                        GenericValue::Map(BTreeMap::from_iter(
                            new_headers
                                .into_iter()
                                .map(|(k, v)| (k, GenericValue::Bytes(v.0))),
                        )),
                    ),
                ]),
            }
            .into());
        }
    };

    log_trace!(body:? = body; "read body");

    if error_on_status && status != 200 {
        return Err(ModuleError {
            causes: vec![common::ErrorKind::STATUS_NOT_OK.into()],
            fatal: true,
            ctx: BTreeMap::from([
                ("url".to_owned(), GenericValue::Str(url.to_owned())),
                ("status".to_string(), GenericValue::Number(status.into())),
                (
                    "headers".to_owned(),
                    GenericValue::Map(BTreeMap::from_iter(
                        new_headers
                            .into_iter()
                            .map(|(k, v)| (k, GenericValue::Bytes(v.0))),
                    )),
                ),
                ("body".to_owned(), body.into()),
            ]),
        }
        .into());
    }

    Ok(ResponseJSON {
        status,
        headers: new_headers,
        body,
    })
}

pub fn try_unwrap_any_err(err: anyhow::Error) -> Result<ModuleError, anyhow::Error> {
    match err.downcast::<ModuleError>() {
        Ok(e) => Ok(e),
        Err(err) => {
            if let Some(e) = err.downcast_ref::<mlua::Error>() {
                ctx::try_unwrap_err(e).ok_or(err)
            } else {
                Err(err)
            }
        }
    }
}
