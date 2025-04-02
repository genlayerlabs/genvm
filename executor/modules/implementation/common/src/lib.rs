use futures_util::{SinkExt, StreamExt};
use std::{future::Future, pin::Pin, sync::Arc};

use anyhow::{Context, Result};
use regex::Regex;
use serde::Deserialize;

pub mod session;

pub type ModuleResult<T> = anyhow::Result<std::result::Result<T, serde_json::Value>>;

pub static CENSOR_RESPONSE: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r#""(set-cookie|cf-ray|access-control[^"]*)": "[^"]*""#).unwrap()
});

fn censor_response(res: &reqwest::Response) -> String {
    let debug = format!("{:?}", res);

    let replacement = |caps: &regex::Captures| -> String {
        format!(r#""{}": "<censored>""#, caps.get(1).unwrap().as_str())
    };

    CENSOR_RESPONSE
        .replace_all(&debug, replacement)
        .into_owned()
}

pub async fn read_response(res: reqwest::Response) -> Result<String> {
    let status = res.status();
    if status != 200 {
        log::error!(response = censor_response(&res), status = status.as_u16(); "request error (1)");
        let text = res.text().await;
        log::error!(body:? = text; "request error (2)");
        return Err(anyhow::anyhow!(
            "request error status={} body={:?}",
            status.as_u16(),
            text,
        ));
    }
    let text = res.text().await.with_context(|| "reading body as text")?;
    log::debug!(body = text; "read response");
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
            .ok_or(anyhow::anyhow!("service closed connection"))??
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
                    Ok(Ok(res)) => genvm_modules_interfaces::Result::Ok(res),
                    Ok(Err(res)) => genvm_modules_interfaces::Result::UserError(res),
                    Err(res) => {
                        log::error!(error = genvm_common::log_error(&res); "handler error");
                        genvm_modules_interfaces::Result::FatalError(format!("{res:#}"))
                    }
                };
                let answer = serde_json::to_string(&res)?;
                let message = Message::Text(answer.into());

                stream.send(message).await?;
            }
        }
    }
}

async fn loop_one_impl<T, R>(
    handler_provider: Arc<impl MessageHandlerProvider<T, R>>,
    stream: tokio::net::TcpStream,
) -> anyhow::Result<()>
where
    T: serde::de::DeserializeOwned + 'static,
    R: serde::Serialize + Send + 'static,
{
    let mut stream = tokio_tungstenite::accept_async(stream).await?;

    let mut handler = handler_provider.new_handler().await?;

    let res = loop_one_inner(&mut handler, &mut stream).await;

    if let Err(close) = handler.cleanup().await {
        log::error!(error = genvm_common::log_error(&close); "cleanup error");
    }

    if res.is_err() {
        if let Err(close) = stream.close(None).await {
            log::error!(error:err = close; "stream closing error")
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
    log::debug!("peer accepted");
    if let Err(e) = loop_one_impl(handler_provider, stream).await {
        log::error!(error = genvm_common::log_error(&e); "internal loop error");
    }
    log::debug!("peer done");
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
