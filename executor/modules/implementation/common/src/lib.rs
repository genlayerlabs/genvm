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

async fn loop_one_inner<T, R>(
    cancel: Arc<genvm_common::cancellation::Token>,
    handler: &mut impl MessageHandler<T, R>,
    mut stream: WSStream,
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
                let payload = serde_json::from_str(text.as_str())?;
                let res = handler.handle(payload).await;
                let res = res?; // not correct
                let answer = serde_json::to_string(&res)?;
                stream.send(Message::Text(answer.into())).await?;
            }
        }
    }
}

async fn loop_one_impl<T, R>(
    cancel: Arc<genvm_common::cancellation::Token>,
    handler_provider: Arc<impl MessageHandlerProvider<T, R>>,
    stream: tokio::net::TcpStream,
) -> anyhow::Result<()>
where
    T: serde::de::DeserializeOwned + 'static,
    R: serde::Serialize + Send + 'static,
{
    let stream = tokio_tungstenite::accept_async(stream).await?;

    let mut handler = handler_provider.new_handler().await?;

    let res = loop_one_inner(cancel, &mut handler, stream).await;

    if let Err(close) = handler.cleanup().await {
        log::error!(error:? = close; "cleanup error");
    }

    res
}

async fn loop_one<T, R>(
    cancel: Arc<genvm_common::cancellation::Token>,
    handler_provider: Arc<impl MessageHandlerProvider<T, R>>,
    stream: tokio::net::TcpStream,
) where
    T: serde::de::DeserializeOwned + 'static,
    R: serde::Serialize + Send + 'static,
{
    if let Err(e) = loop_one_impl(cancel, handler_provider, stream).await {
        log::error!(error:? = e; "internal loop error");
    }
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

    while let Ok((stream, _)) = listener.accept().await {
        tokio::spawn(loop_one(cancel.clone(), handler_provider.clone(), stream));
    }

    Ok(())
}
