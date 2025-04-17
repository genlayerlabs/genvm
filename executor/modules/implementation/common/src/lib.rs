use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;

use anyhow::{Context, Result};
use regex::Regex;

pub mod session;

#[derive(Debug)]
pub struct ModuleResultUserError(pub serde_json::Value);

impl std::fmt::Display for ModuleResultUserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ModuleResultUserError")
    }
}

impl std::error::Error for ModuleResultUserError {}

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
        log::error!(response = censor_debug(&res), status = status.as_u16(), cookie = get_cookie(); "request error (1)");
        let text = res.text().await;
        log::error!(body:? = text, cookie = get_cookie(); "request error (2)");
        return Err(anyhow::anyhow!(
            "request error status={} body={:?}",
            status.as_u16(),
            text,
        ));
    }
    let text = res.text().await.with_context(|| "reading body as text")?;

    if log::log_enabled!(log::Level::Debug) {
        match serde_json::from_str::<serde_json::Value>(&text) {
            Ok(val) => {
                log::debug!(body_json:serde = val, cookie = get_cookie(); "read response");
            }
            Err(_) => {
                log::debug!(body_text = text, cookie = get_cookie(); "read response");
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
    text: &str,
) -> ModuleResult<R>
where
    T: serde::de::DeserializeOwned + 'static,
{
    let payload = serde_json::from_str(text).with_context(|| "parsing payload")?;
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
                let text = x.into_text()?;
                let res = loop_one_inner_handle(handler, text.as_str()).await;
                let res = match res {
                    Ok(res) => genvm_modules_interfaces::Result::Ok(res),
                    Err(res) => match res.downcast::<ModuleResultUserError>() {
                        Ok(ModuleResultUserError(res)) => {
                            log::info!(error:serde = res, cookie = cookie; "handler user error");
                            genvm_modules_interfaces::Result::UserError(res)
                        }
                        Err(res) => {
                            log::error!(error = genvm_common::log_error(&res), cookie = cookie; "handler fatal error");
                            genvm_modules_interfaces::Result::FatalError(format!("{res:#}"))
                        }
                    },
                };

                let answer = serde_json::to_string(&res)?;
                let message = Message::Text(answer.into());

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
                let text = x.into_text()?;
                let genvm_hello: genvm_modules_interfaces::GenVMHello =
                    serde_json::from_str(text.as_str())?;

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
        log::error!(error = genvm_common::log_error(&close), cookie = cookie; "cleanup error");
    }

    if res.is_err() {
        if let Err(close) = stream.close(None).await {
            log::error!(error:err = close, cookie = cookie; "stream closing error")
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
    log::trace!("sock -> ws upgrade");
    let mut stream = match tokio_tungstenite::accept_async(stream).await {
        Err(e) => {
            let e = e.into();
            log::error!(error = genvm_common::log_error(&e); "accept failed");
            return;
        }
        Ok(stream) => stream,
    };

    log::trace!("reading hello");
    let hello = match read_hello(&mut stream).await {
        Err(e) => {
            log::error!(error = genvm_common::log_error(&e); "read hello failed");
            return;
        }
        Ok(None) => return,
        Ok(Some(hello)) => hello,
    };

    log::trace!(hello:serde = hello; "read hello");

    let cookie = hello.cookie.clone();
    let cookie: &str = &cookie;
    COOKIE.scope(Arc::from(cookie), async {
        log::debug!(cookie = cookie; "peer accepted");
        if let Err(e) = loop_one_impl(handler_provider, &mut stream, hello).await {
            log::error!(error = genvm_common::log_error(&e), cookie = cookie; "internal loop error");
        }
        log::debug!(cookie = cookie; "peer done");
    }).await;
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
    let listener = tokio::net::TcpListener::bind(&bind_address).await?;

    log::info!(address = bind_address; "loop started");

    loop {
        tokio::select! {
            _ = cancel.chan.closed() => {
                log::info!("loop cancelled");
                return Ok(())
            }
            accepted = listener.accept() => {
                if let Ok((stream, _)) = accepted {
                    tokio::spawn(loop_one(handler_provider.clone(), stream));
                } else {
                    log::info!("accepted None");
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

pub fn test_with_cookie<F>(value: &str, f: F) -> tokio::task::futures::TaskLocalFuture<Arc<str>, F>
where
    F: std::future::Future,
{
    COOKIE.scope(Arc::from(value), f)
}
