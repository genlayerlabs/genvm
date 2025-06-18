use futures_util::{SinkExt, StreamExt};
use genvm_common::*;
use genvm_modules_interfaces::GenericValue;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Arc};

use anyhow::{Context, Result};
use regex::Regex;

use crate::scripting;

#[allow(non_camel_case_types, dead_code)]
pub enum ErrorKind {
    STATUS_NOT_OK,
    READING_BODY,
    SENDING_REQUEST,
    DESERIALIZING,
    ABSENT_HEADER,
    Other(String),
}

impl From<ErrorKind> for String {
    fn from(x: ErrorKind) -> String {
        if let ErrorKind::Other(k) = x {
            k
        } else {
            x.to_string()
        }
    }
}

impl ErrorKind {
    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        match self {
            ErrorKind::STATUS_NOT_OK => "STATUS_NOT_OK".to_owned(),
            ErrorKind::READING_BODY => "READING_BODY".to_owned(),
            ErrorKind::SENDING_REQUEST => "SENDING_REQUEST".to_owned(),
            ErrorKind::DESERIALIZING => "DESERIALIZING".to_owned(),
            ErrorKind::ABSENT_HEADER => "ABSENT_HEADER".to_owned(),
            ErrorKind::Other(str) => str.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModuleError {
    pub causes: Vec<String>,
    pub fatal: bool,
    pub ctx: BTreeMap<String, genvm_modules_interfaces::GenericValue>,
}

impl ModuleError {
    pub fn try_unwrap_dyn(
        err: &(dyn std::error::Error + Send + Sync + 'static),
    ) -> Option<ModuleError> {
        if let Some(e) = err.downcast_ref::<ModuleError>() {
            return Some(e.clone());
        }

        None
    }
}

#[derive(Serialize, Deserialize)]
pub struct ModuleBaseConfig {
    pub bind_address: String,

    pub lua_script_path: String,
    pub vm_count: usize,
    pub extra_lua_path: String,

    pub signer_url: Arc<str>,
    pub signer_headers: Arc<BTreeMap<String, String>>,
}

pub trait MapUserError {
    type Output;

    fn map_user_error(
        self,
        message: impl Into<String>,
        fatal: bool,
    ) -> Result<Self::Output, anyhow::Error>;
}

impl<T, E> MapUserError for Result<T, E>
where
    E: Into<anyhow::Error>,
{
    type Output = T;

    fn map_user_error(
        self,
        message: impl Into<String>,
        fatal: bool,
    ) -> Result<Self::Output, anyhow::Error> {
        match self {
            Ok(s) => Ok(s),
            Err(e) => {
                let e = e.into();
                match e.downcast::<ModuleError>() {
                    Ok(mut e) => {
                        e.causes.insert(0, message.into());
                        Err(ModuleError {
                            causes: e.causes,
                            fatal: fatal || e.fatal,
                            ctx: e.ctx,
                        }
                        .into())
                    }
                    Err(e) => Err(ModuleError {
                        causes: vec![message.into()],
                        fatal,
                        ctx: BTreeMap::from([(
                            "rust_error".to_owned(),
                            genvm_modules_interfaces::GenericValue::Str(format!("{e:#}")),
                        )]),
                    }
                    .into()),
                }
            }
        }
    }
}

impl std::fmt::Display for ModuleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", self))
    }
}

impl std::error::Error for ModuleError {}

pub type ModuleResult<T> = anyhow::Result<T>;

static CENSOR_RESPONSE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r#""[^"]*(authorization|key|set-cookie|cf-ray|access-control)[^"]*": "[^"]*""#)
        .unwrap()
});

pub fn censor_str(debug: &str) -> String {
    let debug = debug.to_lowercase();

    let replacement = |caps: &regex::Captures| -> String {
        format!(r#""{}": "<censored>""#, caps.get(1).unwrap().as_str())
    };

    CENSOR_RESPONSE
        .replace_all(&debug, replacement)
        .into_owned()
}

pub fn censor_debug(res: &impl std::fmt::Debug) -> String {
    let debug = format!("{:?}", res);

    censor_str(&debug)
}

pub async fn read_response(res: reqwest::Response) -> Result<String> {
    let status = res.status();
    if status != 200 {
        log_error!(response = censor_debug(&res), status = status.as_u16(), cookie = get_cookie(); "request error (1)");
        let text = res.text().await;
        log_error!(body:? = text, cookie = get_cookie(); "request error (2)");
        return Err(anyhow::anyhow!(
            "request error status={} body={:?}",
            status.as_u16(),
            text,
        ));
    }
    let text = res.text().await.with_context(|| "reading body as text")?;

    if log_enabled!(logger::Level::Debug) {
        match serde_json::from_str::<serde_json::Value>(&text) {
            Ok(val) => {
                log_debug!(body_json:serde = val, cookie = get_cookie(); "read response");
            }
            Err(_) => {
                log_debug!(body_text = text, cookie = get_cookie(); "read response");
            }
        }
    }

    Ok(text)
}

type WSStream = tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>;

pub trait MessageHandler<T, R>: Sync + Send {
    fn handle(&self, v: T) -> impl std::future::Future<Output = ModuleResult<R>> + Send;
    fn cleanup(&self) -> impl std::future::Future<Output = anyhow::Result<()>> + Send;
}

pub trait MessageHandlerProvider<T, R>: Sync + Send {
    fn new_handler(
        &self,
        hello: genvm_modules_interfaces::GenVMHello,
    ) -> impl std::future::Future<Output = anyhow::Result<impl MessageHandler<T, R>>> + Send;
}

async fn loop_one_inner_handle<T, R>(
    handler: &mut impl MessageHandler<T, R>,
    text: &[u8],
) -> ModuleResult<R>
where
    T: serde::de::DeserializeOwned + 'static,
{
    let payload = genvm_common::calldata::decode(text)
        .with_context(|| format!("parsing calldata format {:?}", text))?;
    let payload =
        genvm_common::calldata::from_value(payload).with_context(|| "parsing calldata value")?;
    handler.handle(payload).await.with_context(|| "handling")
}

async fn loop_one_inner<T, R>(
    handler: &mut impl MessageHandler<T, R>,
    stream: &mut WSStream,
    cookie: &str,
) -> anyhow::Result<()>
where
    T: serde::de::DeserializeOwned + 'static,
    R: serde::Serialize + Send + 'static,
{
    loop {
        use tokio_tungstenite::tungstenite::Message;

        match stream
            .next()
            .await
            .ok_or_else(|| anyhow::anyhow!("service closed connection"))??
        {
            Message::Ping(v) => {
                stream.send(Message::Pong(v)).await?;
            }
            Message::Pong(_) => {}
            Message::Close(_) => return Ok(()),
            x => {
                let text = x.into_data();
                let res = loop_one_inner_handle(handler, &text).await;
                let res = match res {
                    Ok(res) => genvm_modules_interfaces::Result::Ok(res),
                    Err(err) => match scripting::try_unwrap_any_err(err) {
                        Ok(err) => {
                            if err.fatal {
                                genvm_modules_interfaces::Result::FatalError(format!("{err:#}"))
                            } else {
                                let res = GenericValue::Map(BTreeMap::from([
                                    (
                                        "causes".to_owned(),
                                        GenericValue::Array(
                                            err.causes.into_iter().map(Into::into).collect(),
                                        ),
                                    ),
                                    ("ctx".to_owned(), GenericValue::Map(err.ctx)),
                                ]));
                                genvm_modules_interfaces::Result::UserError(res)
                            }
                        }
                        Err(err) => {
                            log_error!(error:ah = &err, cookie = cookie; "handler fatal error");
                            genvm_modules_interfaces::Result::FatalError(format!("{err:#}"))
                        }
                    },
                };

                let answer = genvm_common::calldata::to_value(&res)?;
                let message = Message::Binary(genvm_common::calldata::encode(&answer).into());

                stream.send(message).await?;
            }
        }
    }
}

async fn read_hello(
    stream: &mut WSStream,
) -> anyhow::Result<Option<genvm_modules_interfaces::GenVMHello>> {
    loop {
        use tokio_tungstenite::tungstenite::Message;
        match stream
            .next()
            .await
            .ok_or_else(|| anyhow::anyhow!("connection closed"))??
        {
            Message::Ping(v) => {
                stream.send(Message::Pong(v)).await?;
            }
            Message::Pong(_) => {}
            Message::Close(_) => return Ok(None),
            x => {
                let text = x.into_data();

                let genvm_hello = genvm_common::calldata::decode(&text)?;
                let genvm_hello: genvm_modules_interfaces::GenVMHello =
                    genvm_common::calldata::from_value(genvm_hello)?;

                return Ok(Some(genvm_hello));
            }
        }
    }
}

async fn loop_one_impl<T, R>(
    handler_provider: Arc<impl MessageHandlerProvider<T, R>>,
    stream: &mut WSStream,
    hello: genvm_modules_interfaces::GenVMHello,
) -> anyhow::Result<()>
where
    T: serde::de::DeserializeOwned + 'static,
    R: serde::Serialize + Send + 'static,
{
    let cookie = hello.cookie.clone();

    let mut handler = handler_provider.new_handler(hello).await?;

    let res = loop_one_inner(&mut handler, stream, &cookie).await;

    if let Err(close) = handler.cleanup().await {
        log_error!(error:ah = &close, cookie = cookie; "cleanup error");
    }

    if res.is_err() {
        if let Err(close) = stream.close(None).await {
            log_error!(error:err = close, cookie = cookie; "stream closing error")
        }
    }

    res
}

async fn loop_one<T, R>(
    handler_provider: Arc<impl MessageHandlerProvider<T, R>>,
    stream: tokio::net::TcpStream,
) where
    T: serde::de::DeserializeOwned + 'static,
    R: serde::Serialize + Send + 'static,
{
    log_trace!("sock -> ws upgrade");
    let mut stream = match tokio_tungstenite::accept_async(stream).await {
        Err(e) => {
            let e = e.into();
            log_error!(error:ah = &e; "accept failed");
            return;
        }
        Ok(stream) => stream,
    };

    log_trace!("reading hello");
    let hello = match read_hello(&mut stream).await {
        Err(e) => {
            log_error!(error:ah = &e; "read hello failed");
            return;
        }
        Ok(None) => return,
        Ok(Some(hello)) => hello,
    };

    log_trace!(hello:serde = hello; "read hello");

    let cookie = hello.cookie.clone();
    let cookie: &str = &cookie;
    COOKIE
        .scope(Arc::from(cookie), async {
            log_debug!(cookie = cookie; "peer accepted");
            if let Err(e) = loop_one_impl(handler_provider, &mut stream, hello).await {
                log_error!(error:ah = &e, cookie = cookie; "internal loop error");
            }
            log_debug!(cookie = cookie; "peer done");
        })
        .await;
}

pub async fn run_loop<T, R>(
    bind_address: String,
    cancel: Arc<genvm_common::cancellation::Token>,
    handler_provider: Arc<impl MessageHandlerProvider<T, R> + 'static>,
) -> anyhow::Result<()>
where
    T: serde::de::DeserializeOwned + 'static,
    R: serde::Serialize + Send + 'static,
{
    log_info!(address = bind_address; "trying to bind");

    let listener = tokio::net::TcpListener::bind(&bind_address).await?;

    log_info!(address = bind_address, local_address:? = listener.local_addr(); "loop started");

    loop {
        tokio::select! {
            _ = cancel.chan.closed() => {
                log_info!("loop cancelled");
                return Ok(())
            }
            accepted = listener.accept() => {
                if let Ok((stream, _)) = accepted {
                    tokio::spawn(loop_one(handler_provider.clone(), stream));
                } else {
                    log_info!("accepted None");
                    return Ok(())
                }
            }
        }
    }
}

tokio::task_local! {
    static COOKIE: Arc<str>;
}

pub fn get_cookie() -> Arc<str> {
    match COOKIE.try_with(|f| f.clone()) {
        Ok(v) => v,
        Err(_) => Arc::from("<absent>"),
    }
}

#[allow(dead_code)]
pub fn test_with_cookie<F>(value: &str, f: F) -> tokio::task::futures::TaskLocalFuture<Arc<str>, F>
where
    F: std::future::Future,
{
    COOKIE.scope(Arc::from(value), f)
}

pub fn create_client() -> anyhow::Result<reqwest::Client> {
    reqwest::ClientBuilder::new()
        .user_agent("reqwest")
        .build()
        .map_err(Into::into)
}

pub fn setup_cancels(
    rt: &tokio::runtime::Runtime,
    die_with_parent: bool,
) -> anyhow::Result<Arc<cancellation::Token>> {
    let (token, canceller) = genvm_common::cancellation::make();

    let canceller_cloned = canceller.clone();
    let handle_sigterm = move || {
        // unfortunately, we cannot log here, as log may `malloc` and it will lead to deadlock
        // in case signal handler is invoked from `malloc` itself...
        canceller_cloned();
    };
    unsafe {
        signal_hook::low_level::register(signal_hook::consts::SIGTERM, handle_sigterm.clone())?;
        signal_hook::low_level::register(signal_hook::consts::SIGINT, handle_sigterm)?;
    }

    if die_with_parent {
        let parent_pid = std::os::unix::process::parent_id();
        let token = token.clone();

        log_info!(parent_pid = parent_pid; "monitoring parent pid to exit when it changes");

        rt.spawn(async move {
            loop {
                tokio::select! {
                   _ = tokio::time::sleep(tokio::time::Duration::from_secs(5)) => {
                        let new_parent_pid = std::os::unix::process::parent_id();
                        if new_parent_pid == parent_pid {
                            continue;
                        }

                        log_warn!(old = parent_pid, new_parent_pid = new_parent_pid; "parent pid changed, closing");
                        canceller();
                   },
                   _ = token.chan.closed() => {
                        break;
                   },
                };
            }
        });
    }

    Ok(token)
}

#[cfg(test)]
pub mod tests {
    use std::sync::{Arc, Once};

    use genvm_common::logger;

    static INIT: Once = Once::new();

    pub fn setup() {
        INIT.call_once(|| {
            let base_conf = genvm_common::BaseConfig {
                blocking_threads: 0,
                log_disable: Default::default(),
                log_level: logger::Level::Trace,
                threads: 0,
            };
            base_conf.setup_logging(std::io::stdout()).unwrap();
        });
    }

    pub fn get_hello() -> Arc<genvm_modules_interfaces::GenVMHello> {
        Arc::new(genvm_modules_interfaces::GenVMHello {
            cookie: "test_cookie".to_owned(),
            host_data: genvm_modules_interfaces::HostData {
                node_address: "test_node_address".to_owned(),
                tx_id: "test_tx_id".to_owned(),
                rest: serde_json::Map::new(),
            },
        })
    }
}
