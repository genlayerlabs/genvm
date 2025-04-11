use std::sync::Arc;

use anyhow::Context;
use futures_util::{stream::FusedStream, SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::{Message, Utf8Bytes};

type WSStream =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

struct ModuleImpl {
    url: String,
    stream: Option<WSStream>,
}

pub struct Module {
    name: String,
    cancellation: Arc<genvm_common::cancellation::Token>,
    imp: tokio::sync::Mutex<ModuleImpl>,
    cookie: String,
}

async fn read_handling_pings(stream: &mut WSStream) -> anyhow::Result<Utf8Bytes> {
    loop {
        match stream
            .next()
            .await
            .ok_or_else(|| anyhow::anyhow!("service closed connection"))??
        {
            Message::Ping(v) => {
                stream.send(Message::Pong(v)).await?;
            }
            Message::Pong(_) => {}
            Message::Close(_) => anyhow::bail!("stream closed"),
            Message::Text(text) => return Ok(text),
            x => {
                log::info!(payload:? = x; "received unexpected");
                let text = x.into_text()?;
                return Ok(text);
            }
        }
    }
}

impl Module {
    pub fn new(
        name: String,
        url: String,
        cancellation: Arc<genvm_common::cancellation::Token>,
        cookie: String,
    ) -> Self {
        Self {
            imp: tokio::sync::Mutex::new(ModuleImpl { url, stream: None }),
            cancellation,
            cookie,
            name,
        }
    }

    pub async fn close(&self) {
        let mut lock = self.imp.lock().await;
        if let Some(stream) = &mut lock.stream {
            if stream.is_terminated() {
                return;
            }
            if let Err(e) = stream.close(None).await {
                log::error!(error:err = e; "closing stream");
            }
        }
    }

    async fn send_impl<R, V>(
        &self,
        val: V,
    ) -> anyhow::Result<std::result::Result<R, serde_json::Value>>
    where
        V: serde::Serialize,
        R: serde::Serialize + serde::de::DeserializeOwned,
    {
        let mut zelf = self.imp.lock().await;

        if zelf.stream.is_none() {
            let (mut ws_stream, _) = tokio_tungstenite::connect_async(&zelf.url).await?;

            let msg = serde_json::to_string(&genvm_modules_interfaces::GenVMHello {
                cookie: self.cookie.clone(),
            })?;
            ws_stream.send(Message::Text(msg.into())).await?;

            zelf.stream = Some(ws_stream);
        }

        match &mut zelf.stream {
            None => unreachable!(),
            Some(stream) => {
                let payload = serde_json::to_string(&val)?;
                stream.send(Message::Text(payload.into())).await?;
                let response = read_handling_pings(stream).await?;

                let res: genvm_modules_interfaces::Result<R> =
                    serde_json::from_str(&response).with_context(|| "parsing result of module")?;

                log::info!(name = self.name, question:serde = val, response:serde = res; "answer from module");

                match res {
                    genvm_modules_interfaces::Result::Ok(v) => Ok(Ok(v)),
                    genvm_modules_interfaces::Result::UserError(value) => Ok(Err(value)),
                    genvm_modules_interfaces::Result::FatalError(value) => {
                        log::error!(error = value; "module error");
                        Err(anyhow::anyhow!("module error: {value}"))
                    }
                }
            }
        }
    }

    pub async fn send<R, V>(
        &self,
        val: V,
    ) -> anyhow::Result<std::result::Result<R, serde_json::Value>>
    where
        V: serde::Serialize,
        R: serde::Serialize + serde::de::DeserializeOwned,
    {
        tokio::select! {
            _ = self.cancellation.chan.closed() => {
                anyhow::bail!("timeout")
            }
            res = self.send_impl(val) => {
                res
            }
        }
    }
}
