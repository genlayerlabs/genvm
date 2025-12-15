use base64::Engine;
use futures_util::{SinkExt, StreamExt};
use genvm_common::*;
use genvm_modules_interfaces::GenericValue;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Arc};

use anyhow::{Context, Result};

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
    pub lua_path: String,

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

    fn map_user_error_module(
        self,
        message: impl Into<String>,
        fatal: bool,
    ) -> Result<Self::Output, ModuleError>;
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
        self.map_user_error_module(message, fatal)
            .map_err(Into::into)
    }

    fn map_user_error_module(
        self,
        message: impl Into<String>,
        fatal: bool,
    ) -> Result<Self::Output, ModuleError> {
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
                        })
                    }
                    Err(e) => Err(ModuleError {
                        causes: vec![message.into()],
                        fatal,
                        ctx: BTreeMap::from([(
                            "rust_error".to_owned(),
                            genvm_modules_interfaces::GenericValue::Str(format!("{e:#}")),
                        )]),
                    }),
                }
            }
        }
    }
}

impl std::fmt::Display for ModuleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{self:?}"))
    }
}

impl std::error::Error for ModuleError {}

pub type ModuleResult<T> = anyhow::Result<T>;

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
        .with_context(|| format!("parsing calldata format {text:?}"))?;
    let payload =
        genvm_common::calldata::from_value(payload).with_context(|| "parsing calldata value")?;
    handler.handle(payload).await.with_context(|| "handling")
}

async fn loop_one_inner<T, R>(
    handler: &mut impl MessageHandler<T, R>,
    stream: &mut WSStream,
    genvm_id: genvm_modules_interfaces::GenVMId,
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
                            log_error_into!(&LoggerWithId, error:ah = &err, genvm_id:id = genvm_id.0; "handler fatal error");
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
    let genvm_id = hello.genvm_id;

    let mut handler = handler_provider.new_handler(hello).await?;

    let res = loop_one_inner(&mut handler, stream, genvm_id).await;

    if let Err(close) = handler.cleanup().await {
        log_error_into!(&LoggerWithId, error:ah = &close, genvm_id:id = genvm_id.0; "cleanup error");
    }

    if res.is_err() {
        if let Err(close) = stream.close(None).await {
            log_error_into!(&LoggerWithId, error:err = close, genvm_id:id = genvm_id.0; "stream closing error")
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

    let genvm_id = hello.genvm_id;
    GENVM_ID
        .scope(genvm_id, async {
            log_debug_into!(&LoggerWithId, genvm_id:id = genvm_id.0; "peer accepted");
            if let Err(e) = loop_one_impl(handler_provider, &mut stream, hello).await {
                log_error_into!(&LoggerWithId, error:ah = &e, genvm_id:id = genvm_id.0; "internal loop error");
            }
            log_debug_into!(&LoggerWithId, genvm_id:id = genvm_id.0; "peer done");
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
    static GENVM_ID: genvm_modules_interfaces::GenVMId;
}

pub fn get_genvm_id() -> genvm_modules_interfaces::GenVMId {
    match GENVM_ID.try_with(|f| *f) {
        Ok(v) => v,
        Err(_) => genvm_modules_interfaces::GenVMId(0), // Use 0 as absent/default value
    }
}

// Keep for backward compatibility
pub fn get_cookie() -> Arc<str> {
    Arc::from(get_genvm_id().to_string())
}

#[allow(dead_code)]
pub fn test_with_cookie<F>(
    value: &str,
    f: F,
) -> tokio::task::futures::TaskLocalFuture<genvm_modules_interfaces::GenVMId, F>
where
    F: std::future::Future,
{
    // Parse the string as a u64 for the genvm_id, fallback to 0 if parsing fails
    let genvm_id = value.parse::<u64>().unwrap_or(0);
    GENVM_ID.scope(genvm_modules_interfaces::GenVMId(genvm_id), f)
}

#[allow(dead_code)]
pub fn test_with_genvm_id<F>(
    genvm_id: genvm_modules_interfaces::GenVMId,
    f: F,
) -> tokio::task::futures::TaskLocalFuture<genvm_modules_interfaces::GenVMId, F>
where
    F: std::future::Future,
{
    GENVM_ID.scope(genvm_id, f)
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

pub enum LogSinkElement {
    Map(serde_json::Map<String, serde_json::Value>),
    Line(String),
    Raw(Vec<u8>),
}

impl LogSinkElement {
    pub fn into_json(self) -> serde_json::Map<String, serde_json::Value> {
        match self {
            LogSinkElement::Map(v) => v,
            LogSinkElement::Line(text) => serde_json::Map::from_iter([
                ("level".into(), serde_json::Value::String("info".into())),
                (
                    "message".into(),
                    serde_json::Value::String("genvm log".into()),
                ),
                ("line".into(), text.into()),
            ]),
            LogSinkElement::Raw(s) => {
                if let Ok(v) = serde_json::from_slice(&s) {
                    v
                } else {
                    let mut as_encoded = String::new();
                    base64::prelude::BASE64_STANDARD.encode_string(s, &mut as_encoded);
                    serde_json::Map::from_iter([
                        ("level".into(), serde_json::Value::String("error".into())),
                        (
                            "message".into(),
                            serde_json::Value::String("genvm log".into()),
                        ),
                        ("line".into(), as_encoded.into()),
                    ])
                }
            }
        }
    }
}

pub type LogSink = Arc<crossbeam::queue::SegQueue<LogSinkElement>>;

pub static GENVM_BY_ID_LOGGER: std::sync::LazyLock<
    papaya::HashMap<genvm_modules_interfaces::GenVMId, LogSink>,
> = std::sync::LazyLock::new(Default::default);

pub struct LoggerWithId;

fn get_logger_sink(record: &genvm_common::logger::Record<'_>) -> Option<LogSink> {
    let Some((_, genvm_common::logger::Capture::Id(genvm_id))) =
        record.kv.iter().find(|x| x.0 == "genvm_id")
    else {
        return None;
    };
    let genvm_id = *genvm_id;

    GENVM_BY_ID_LOGGER
        .pin()
        .get(&genvm_modules_interfaces::GenVMId(genvm_id))
        .cloned()
}

impl genvm_common::logger::ILogger for LoggerWithId {
    fn try_log(
        &self,
        record: genvm_common::logger::Record<'_>,
    ) -> std::result::Result<(), genvm_common::logger::Error> {
        let Some(sink) = get_logger_sink(&record) else {
            if let Some(l) = genvm_common::logger::__LOGGER.get() {
                return l.try_log(record);
            } else {
                return Ok(());
            }
        };

        let mut buf = Vec::new();
        genvm_common::logger::log_into_buffer(&mut buf, record)?;

        sink.push(LogSinkElement::Raw(buf));

        Ok(())
    }

    fn enabled(&self, callsite: genvm_common::logger::Callsite) -> bool {
        if let Some(l) = genvm_common::logger::__LOGGER.get() {
            l.enabled(callsite)
        } else {
            false
        }
    }
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
            genvm_id: genvm_modules_interfaces::GenVMId(999),
            host_data: genvm_modules_interfaces::HostData {
                node_address: "test_node_address".to_owned(),
                tx_id: "test_tx_id".to_owned(),
                rest: serde_json::Map::new(),
            },
        })
    }
}
