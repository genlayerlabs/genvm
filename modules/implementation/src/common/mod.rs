use base64::Engine;
use genvm_common::*;
use genvm_modules_interfaces::GenericValue;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Arc};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

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

#[derive(Clone, Debug)]
pub struct BudgetExhausted;

impl std::fmt::Display for BudgetExhausted {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "INSUFFICIENT_EXECUTION_BUDGET")
    }
}

impl std::error::Error for BudgetExhausted {}

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
    #[serde(default)]
    pub bind_address: Option<String>,

    pub lua_script_path: String,
    pub vm_count: usize,
    pub lua_path: String,

    pub signer_url: Arc<str>,
    pub signer_headers: Arc<BTreeMap<String, String>>,

    pub data_dir: String,
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

pub trait WithGenVMId {
    fn genvm_id(&self) -> genvm_modules_interfaces::GenVMId;
}

pub trait MessageHandler<T, R>: Sync + Send {
    fn handle(&self, v: T) -> impl std::future::Future<Output = ModuleResult<R>> + Send;
    fn cleanup(&self) -> impl std::future::Future<Output = anyhow::Result<()>> + Send;
}

pub trait MessageHandlerProvider<T, R>: Sync + Send {
    type Ctx: WithGenVMId + Send + Sync + 'static;

    fn create_execution_context(
        &self,
        hello: genvm_modules_interfaces::GenVMHello,
    ) -> anyhow::Result<genvm_common::sync::DArc<Self::Ctx>>;

    fn new_handler(
        &self,
        ctx: genvm_common::sync::DArc<Self::Ctx>,
    ) -> impl std::future::Future<Output = anyhow::Result<impl MessageHandler<T, R>>> + Send;
}

/// Write a length-prefixed message to a stream
/// Format: 4 bytes big-endian length + body
pub(crate) async fn write_message<S: tokio::io::AsyncWrite + Unpin>(
    stream: &mut S,
    data: &[u8],
) -> anyhow::Result<()> {
    let len = data.len() as u32;
    stream
        .write_all(&len.to_be_bytes())
        .await
        .context("writing message length")?;
    stream
        .write_all(data)
        .await
        .context("writing message body")?;
    stream.flush().await.context("flushing stream")?;
    Ok(())
}

/// Read a length-prefixed message from a stream
/// Format: 4 bytes big-endian length + body
/// Returns None if the stream is closed (EOF on length read)
pub(crate) async fn read_message<S: tokio::io::AsyncRead + Unpin>(
    stream: &mut S,
) -> anyhow::Result<Option<Vec<u8>>> {
    let mut len_buf = [0u8; 4];
    match stream.read_exact(&mut len_buf).await {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e.into()),
    }
    let len = u32::from_be_bytes(len_buf) as usize;

    let mut data = vec![0u8; len];
    stream.read_exact(&mut data).await?;
    Ok(Some(data))
}

async fn loop_one_inner_handle<T, R>(
    handler: &mut impl MessageHandler<T, R>,
    text: &[u8],
) -> ModuleResult<R>
where
    T: calldata::codec::Decode + 'static,
{
    let payload = genvm_common::calldata::decode_obj(text)
        .with_context(|| format!("parsing calldata format {text:?}"))?;
    handler
        .handle(payload)
        .await
        .with_context(|| "handling with handler")
}

async fn loop_one_inner<T, R, S>(
    handler: &mut impl MessageHandler<T, R>,
    stream: &mut S,
    genvm_id: genvm_modules_interfaces::GenVMId,
) -> anyhow::Result<()>
where
    T: calldata::codec::Decode + 'static,
    R: calldata::codec::Encode<Vec<u8>, Error = std::convert::Infallible> + Send + 'static,
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    loop {
        let Some(data) = read_message(stream)
            .await
            .context("reading module message")?
        else {
            // Stream closed
            return Ok(());
        };

        let res = loop_one_inner_handle(handler, &data).await;
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

        let message = genvm_common::calldata::encode_obj(&res);

        write_message(stream, &message)
            .await
            .context("writing message")?;
    }
}

async fn read_hello<S>(
    stream: &mut S,
) -> anyhow::Result<Option<genvm_modules_interfaces::GenVMHello>>
where
    S: tokio::io::AsyncRead + Unpin,
{
    let Some(data) = read_message(stream)
        .await
        .context("reading module message")?
    else {
        return Ok(None);
    };

    let genvm_hello: genvm_modules_interfaces::GenVMHello =
        genvm_common::calldata::decode_obj(&data).context("decoding GenVMHello")?;

    Ok(Some(genvm_hello))
}

async fn loop_one_impl<T, R, S, P: MessageHandlerProvider<T, R>>(
    handler_provider: Arc<P>,
    stream: &mut S,
    exec_ctx: sync::DArc<P::Ctx>,
) -> anyhow::Result<()>
where
    T: calldata::codec::Decode + 'static,
    R: calldata::codec::Encode<Vec<u8>, Error = std::convert::Infallible> + Send + 'static,
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let genvm_id = exec_ctx.genvm_id();

    let mut handler = handler_provider
        .new_handler(exec_ctx)
        .await
        .context("creating handler")?;

    let res = loop_one_inner(&mut handler, stream, genvm_id)
        .await
        .context("handling");

    if let Err(close) = handler.cleanup().await {
        log_error_into!(&LoggerWithId, error:ah = &close, genvm_id:id = genvm_id.0; "cleanup error");
    }

    res
}

pub async fn handle_stream<T, R, S, P: MessageHandlerProvider<T, R>>(
    handler_provider: Arc<P>,
    mut stream: S,
    stream_type: &str,
    exec_ctx: Option<sync::DArc<P::Ctx>>,
) where
    T: calldata::codec::Decode + 'static,
    R: calldata::codec::Encode<Vec<u8>, Error = std::convert::Infallible> + Send + 'static,
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    log_trace!(stream_type = stream_type; "new connection");

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

    let exec_ctx = if let Some(ctx) = exec_ctx {
        ctx
    } else {
        match handler_provider.create_execution_context(hello) {
            Ok(ctx) => ctx,
            Err(e) => {
                log_error!(error:ah = &e; "failed to create execution context");
                return;
            }
        }
    };

    let genvm_id = exec_ctx.genvm_id();
    GENVM_ID
        .scope(genvm_id, async {
            log_debug_into!(&LoggerWithId, genvm_id:id = genvm_id.0; "peer accepted");
            if let Err(e) = loop_one_impl(handler_provider, &mut stream, exec_ctx).await {
                log_error_into!(&LoggerWithId, error:ah = &e, genvm_id:id = genvm_id.0; "internal loop error");
            }
            log_debug_into!(&LoggerWithId, genvm_id:id = genvm_id.0; "peer done");
        })
        .await;
}

async fn loop_one<T, R, P: MessageHandlerProvider<T, R>>(
    handler_provider: Arc<P>,
    stream: tokio::net::TcpStream,
) where
    T: calldata::codec::Decode + 'static,
    R: calldata::codec::Encode<Vec<u8>, Error = std::convert::Infallible> + Send + 'static,
{
    handle_stream(handler_provider, stream, "tcp", None).await;
}

async fn loop_one_unix<T, R, P: MessageHandlerProvider<T, R>>(
    handler_provider: Arc<P>,
    stream: tokio::net::UnixStream,
) where
    T: calldata::codec::Decode + 'static,
    R: calldata::codec::Encode<Vec<u8>, Error = std::convert::Infallible> + Send + 'static,
{
    handle_stream(handler_provider, stream, "unix", None).await;
}

const UNIX_PREFIX: &str = "unix://";

pub async fn run_loop<T, R, P: MessageHandlerProvider<T, R> + 'static>(
    bind_address: Option<String>,
    cancel: Arc<genvm_common::cancellation::Token>,
    handler_provider: Arc<P>,
) -> anyhow::Result<()>
where
    T: calldata::codec::Decode + 'static,
    R: calldata::codec::Encode<Vec<u8>, Error = std::convert::Infallible> + Send + 'static,
{
    let Some(bind_address) = bind_address else {
        // No bind address - just wait for cancellation
        log_info!("no bind_address configured, waiting for cancellation");
        cancel.chan.closed().await;
        return Ok(());
    };

    if let Some(socket_path) = bind_address.strip_prefix(UNIX_PREFIX) {
        run_loop_unix(socket_path, cancel, handler_provider).await
    } else {
        run_loop_tcp(&bind_address, cancel, handler_provider).await
    }
}

async fn run_loop_tcp<T, R, P: MessageHandlerProvider<T, R> + 'static>(
    bind_address: &str,
    cancel: Arc<genvm_common::cancellation::Token>,
    handler_provider: Arc<P>,
) -> anyhow::Result<()>
where
    T: calldata::codec::Decode + 'static,
    R: calldata::codec::Encode<Vec<u8>, Error = std::convert::Infallible> + Send + 'static,
{
    log_info!(address = bind_address; "trying to bind TCP");

    let listener = tokio::net::TcpListener::bind(bind_address).await?;

    log_info!(address = bind_address, local_address:? = listener.local_addr(); "TCP loop started");

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

async fn run_loop_unix<T, R, P: MessageHandlerProvider<T, R> + 'static>(
    socket_path: &str,
    cancel: Arc<genvm_common::cancellation::Token>,
    handler_provider: Arc<P>,
) -> anyhow::Result<()>
where
    T: calldata::codec::Decode + 'static,
    R: calldata::codec::Encode<Vec<u8>, Error = std::convert::Infallible> + Send + 'static,
{
    let path = std::path::Path::new(socket_path);
    log_info!(socket_path = socket_path; "trying to bind Unix socket");

    // Clean up stale socket
    if path.exists() {
        std::fs::remove_file(path)
            .with_context(|| format!("removing stale socket {}", socket_path))?;
    }

    // Ensure parent directory exists
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }

    let listener = tokio::net::UnixListener::bind(path)
        .with_context(|| format!("binding to unix socket {}", socket_path))?;

    log_info!(socket_path = socket_path; "Unix socket loop started");

    // Note: Socket cleanup on cancellation
    let cleanup_path = path.to_owned();
    let _dropper = sync::DropGuard::new(move || {
        if let Err(e) = std::fs::remove_file(&cleanup_path) {
            log_error!(error:err = &e, socket_path:? = cleanup_path; "cleaning up unix socket failed");
        }
    });

    async {
        loop {
            tokio::select! {
                _ = cancel.chan.closed() => {
                    log_info!("loop cancelled");
                    return Ok(())
                }
                accepted = listener.accept() => {
                    if let Ok((stream, _)) = accepted {
                        tokio::spawn(loop_one_unix(handler_provider.clone(), stream));
                    } else {
                        log_info!("accepted None");
                        return Ok(())
                    }
                }
            }
        }
    }
    .await
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

#[derive(Debug, Clone, Copy)]
pub struct Timeout(std::time::Duration);

impl Timeout {
    pub const fn from_secs(secs: u64) -> Self {
        Timeout(std::time::Duration::from_secs(secs))
    }

    pub fn to_duration(self) -> std::time::Duration {
        self.0
    }

    fn from_secs_f64_checked(n: f64) -> Result<std::time::Duration, String> {
        if !n.is_finite() || n < 0.0 {
            return Err(format!(
                "timeout must be a finite, non-negative number, got {n}"
            ));
        }
        std::time::Duration::try_from_secs_f64(n).map_err(|e| e.to_string())
    }

    fn parse_str(s: &str) -> Result<std::time::Duration, String> {
        if let Some(v) = s.strip_suffix("ms") {
            v.trim()
                .parse::<f64>()
                .map_err(|e| e.to_string())
                .and_then(|n| Self::from_secs_f64_checked(n / 1000.0))
        } else if let Some(v) = s.strip_suffix('m') {
            v.trim()
                .parse::<f64>()
                .map_err(|e| e.to_string())
                .and_then(|n| Self::from_secs_f64_checked(n * 60.0))
        } else if let Some(v) = s.strip_suffix('s') {
            v.trim()
                .parse::<f64>()
                .map_err(|e| e.to_string())
                .and_then(Self::from_secs_f64_checked)
        } else {
            Err(format!("expected suffix m, s, or ms, got \"{s}\""))
        }
    }
}

impl Serialize for Timeout {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_f64(self.0.as_secs_f64())
    }
}

impl<'de> serde::Deserialize<'de> for Timeout {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct TimeoutVisitor;

        impl<'de> serde::de::Visitor<'de> for TimeoutVisitor {
            type Value = Timeout;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("a number (seconds) or a string like \"1.5s\", \"500ms\", \"2m\"")
            }

            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<Timeout, E> {
                Timeout::from_secs_f64_checked(v as f64)
                    .map(Timeout)
                    .map_err(E::custom)
            }

            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<Timeout, E> {
                Ok(Timeout(std::time::Duration::from_secs(v)))
            }

            fn visit_f64<E: serde::de::Error>(self, v: f64) -> Result<Timeout, E> {
                Timeout::from_secs_f64_checked(v)
                    .map(Timeout)
                    .map_err(E::custom)
            }

            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<Timeout, E> {
                Timeout::parse_str(v).map(Timeout).map_err(E::custom)
            }
        }

        deserializer.deserialize_any(TimeoutVisitor)
    }
}

pub fn create_client() -> anyhow::Result<reqwest::Client> {
    reqwest::ClientBuilder::new()
        .user_agent("reqwest")
        .timeout(std::time::Duration::from_secs(300))
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
        genvm_common::logger::log_into_buffer(&mut buf, record, Default::default())?;

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

    pub fn create_test_client() -> reqwest::Client {
        reqwest::ClientBuilder::new()
            .user_agent("reqwest")
            .timeout(std::time::Duration::from_secs(40))
            .build()
            .unwrap()
    }

    pub fn get_hello() -> Arc<genvm_modules_interfaces::GenVMHello> {
        Arc::new(genvm_modules_interfaces::GenVMHello {
            genvm_id: genvm_modules_interfaces::GenVMId(999),
            role: genvm_modules_interfaces::Role::Leader,
            host_data: genvm_modules_interfaces::HostData {
                node_address: "test_node_address".to_owned(),
                tx_id: "test_tx_id".to_owned(),
                rest: serde_json::Map::new(),
            },
            gas_data: std::collections::BTreeMap::new(),
            initial_time_units_allocation: 0,
        })
    }
}
