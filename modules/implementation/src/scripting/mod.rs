pub mod pool;

mod ctx;

use anyhow::Context as _;
use genvm_common::{sync::DArc, *};
use genvm_modules_interfaces::GenericValue;
use mlua::LuaSerdeExt;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, future::Future, sync::Arc};

use crate::common::{self, MapUserError, ModuleError};

pub use ctx::filters;
pub use ctx::CtxPart;
pub use ctx::Metrics;

pub struct LuaDArc<T: 'static>(pub DArc<T>);

impl<T: Send + Sync + 'static> mlua::UserData for LuaDArc<T> {}

impl<T: 'static> std::ops::Deref for LuaDArc<T> {
    type Target = DArc<T>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub type CtxCreator<R, E> =
    dyn Fn(&mlua::Lua, &mlua::Table, &sync::DArc<E>) -> anyhow::Result<R> + Send + Sync;

pub struct UserVM<T, R, E: 'static> {
    pub vm: mlua::Lua,
    pub data: T,

    ctx_creator: Box<CtxCreator<R, E>>,
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

pub fn create_ctx_part(
    hello: &Arc<genvm_modules_interfaces::GenVMHello>,
    base_config: &sync::DArc<common::ModuleBaseConfig>,
    metrics: sync::DArc<Metrics>,
) -> anyhow::Result<ctx::CtxPart> {
    let mut sign_vars = BTreeMap::new();

    sign_vars.insert(
        "node_address".to_owned(),
        hello.host_data.node_address.clone(),
    );
    sign_vars.insert("tx_id".to_owned(), hello.host_data.tx_id.clone());
    for (k, v) in &hello.host_data.rest {
        if let serde_json::Value::String(s) = v {
            sign_vars.insert(k.clone(), s.clone());
        }
    }

    Ok(ctx::CtxPart {
        hello: hello.clone(),
        client: common::create_client()?,
        node_address: hello.host_data.node_address.clone(),
        sign_vars,
        sign_headers: base_config.signer_headers.clone(),
        sign_url: base_config.signer_url.clone(),
        metrics,
    })
}

pub fn setup_lua_default_ctx(
    ctx: sync::DArc<ctx::CtxPart>,
    vm: &mlua::Lua,
    table: &mlua::Table,
) -> anyhow::Result<()> {
    let hello_value = vm.to_value_with(&ctx.hello, DEFAULT_LUA_SER_OPTIONS)?;
    let my_ctx = vm.create_userdata(LuaDArc(ctx))?;
    table.set("__ctx_dflt", my_ctx)?;
    let hello_value = hello_value
        .as_table()
        .ok_or_else(|| mlua::Error::external("expected hello value to be a table"))?;

    for kv in hello_value.pairs() {
        let (k, v): (mlua::Value, mlua::Value) = kv?;
        table.set(k, v)?;
    }

    Ok(())
}

pub fn create_default_ctx(
    hello: &Arc<genvm_modules_interfaces::GenVMHello>,
    base_config: sync::DArc<common::ModuleBaseConfig>,
    metrics: sync::DArc<Metrics>,
    vm: &mlua::Lua,
    table: &mlua::Table,
) -> anyhow::Result<sync::DArc<ctx::CtxPart>> {
    let ctx_part = sync::DArc::new(create_ctx_part(hello, &base_config, metrics)?);
    setup_lua_default_ctx(ctx_part.clone(), vm, table)?;
    Ok(ctx_part)
}

impl<T, R, E> UserVM<T, R, E> {
    pub fn create_ctx(&self, ctx: &sync::DArc<E>) -> anyhow::Result<(R, mlua::Value)> {
        let table = self.vm.create_table()?;

        let res = (self.ctx_creator)(&self.vm, &table, ctx)?;

        Ok((res, mlua::Value::Table(table)))
    }

    pub async fn create<F>(
        mod_config: &common::ModuleBaseConfig,
        data_getter: impl FnOnce(mlua::Lua) -> F,
        ctx_creator: Box<CtxCreator<R, E>>,
    ) -> anyhow::Result<Self>
    where
        F: Future<Output = anyhow::Result<T>>,
    {
        use mlua::StdLib;

        std::env::set_var("LUA_PATH", &mod_config.lua_path);

        let lua_libs = StdLib::COROUTINE
            | StdLib::TABLE
            | StdLib::IO
            | StdLib::STRING
            | StdLib::MATH
            | StdLib::PACKAGE;

        let vm = mlua::Lua::new_with(lua_libs, mlua::LuaOptions::default())?;

        vm.load_std_libs(lua_libs).context("loading stdlib")?;

        vm.globals().set(
            "__dflt",
            ctx::dflt::create_global(&vm).context("creating global for __dflt")?,
        )?;

        Ok(Self {
            data: data_getter(vm.clone()).await?,
            vm,
            ctx_creator,
        })
    }

    pub async fn call_fn<RR>(
        &self,
        f: &mlua::Function,
        args: impl mlua::IntoLuaMulti,
    ) -> anyhow::Result<RR>
    where
        RR: mlua::FromLuaMulti,
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
    P: AsRef<std::path::Path> + Into<String> + std::fmt::Debug,
{
    let script_contents = std::fs::read_to_string(&path)
        .with_context(|| format!("reading script from {:?}", &path))?;
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

    pub headers: BTreeMap<String, bytes::Bytes>,

    #[serde(with = "serde_bytes")]
    pub body: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResponseJSON {
    pub status: u16,

    pub headers: BTreeMap<String, bytes::Bytes>,

    pub body: serde_json::Value,
}

async fn read_response_body_limited(
    mut response: reqwest::Response,
    limit: usize,
) -> anyhow::Result<Vec<u8>> {
    if limit == usize::MAX {
        return Ok(response.bytes().await?.to_vec());
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if body.len() + chunk.len() > limit {
            anyhow::bail!("response body size exceeds limit of {limit} bytes");
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

pub async fn send_request_get_lua_compatible_response_bytes(
    metrics: &DArc<Metrics>,
    url: &str,
    request: reqwest::RequestBuilder,
    error_on_status: bool,
    body_size_limit: usize,
) -> anyhow::Result<Response> {
    metrics.requests_count.increment();
    let lock = stats::tracker::Time::new(metrics.gep(|x| &x.requests_time));

    let response = request
        .send()
        .await
        .map_user_error(common::ErrorKind::SENDING_REQUEST, true)?;

    let status = response.status().as_u16();
    let mut new_headers = BTreeMap::<String, bytes::Bytes>::new();
    for (k, v) in response.headers() {
        new_headers.insert(
            k.as_str().to_owned(),
            bytes::Bytes::copy_from_slice(v.as_bytes()),
        );
    }

    let body = read_response_body_limited(response, body_size_limit).await;
    std::mem::drop(lock);

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
                                .map(|(k, v)| (k, GenericValue::Bytes(v.to_vec()))),
                        )),
                    ),
                ]),
            }
            .into());
        }
    };

    log_trace!(body:bytes = body, len = body.len(); "read body");

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
                            .map(|(k, v)| (k, GenericValue::Bytes(v.to_vec()))),
                    )),
                ),
                ("body".to_owned(), GenericValue::Bytes(body)),
            ]),
        }
        .into());
    }

    Ok(Response {
        status,
        headers: new_headers,
        body,
    })
}

pub async fn send_request_get_lua_compatible_response_json(
    metrics: &DArc<Metrics>,
    url: &str,
    request: reqwest::RequestBuilder,
    error_on_status: bool,
    body_size_limit: usize,
) -> anyhow::Result<ResponseJSON> {
    metrics.requests_count.increment();
    let lock = stats::tracker::Time::new(metrics.gep(|x| &x.requests_time));

    let response = request
        .send()
        .await
        .map_user_error(common::ErrorKind::SENDING_REQUEST, true)?;

    let status = response.status().as_u16();
    let mut new_headers = BTreeMap::<String, bytes::Bytes>::new();
    for (k, v) in response.headers() {
        new_headers.insert(
            k.as_str().to_owned(),
            bytes::Bytes::copy_from_slice(v.as_bytes()),
        );
    }

    let body = read_response_body_limited(response, body_size_limit).await;
    std::mem::drop(lock);

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
                                .map(|(k, v)| (k, GenericValue::Bytes(v.to_vec()))),
                        )),
                    ),
                ]),
            }
            .into());
        }
    };

    let body: serde_json::Value = match serde_json::from_slice(&body) {
        Ok(body) => body,
        Err(e) => {
            return Err(ModuleError {
                causes: vec![common::ErrorKind::DESERIALIZING.into()],
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
                                .map(|(k, v)| (k, GenericValue::Bytes(v.to_vec()))),
                        )),
                    ),
                    ("body".to_owned(), GenericValue::Bytes(body)),
                ]),
            }
            .into());
        }
    };

    log_trace!(body:serde = body; "read body");

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
                            .map(|(k, v)| (k, GenericValue::Bytes(v.to_vec()))),
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
