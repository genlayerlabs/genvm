use genvm_common::*;
use std::sync::Arc;

use anyhow::Context;
use futures_util::{stream::FusedStream, SinkExt, StreamExt};
use genvm_common::calldata;
use genvm_modules_interfaces::GenericValue;
use tokio_tungstenite::tungstenite::{Bytes, Message};

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
    host_data: Arc<serde_json::Value>,
}

async fn read_handling_pings(stream: &mut WSStream) -> anyhow::Result<Bytes> {
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
            Message::Text(text) => return Ok(text.into()),
            Message::Binary(text) => return Ok(text),
            x => {
                log_info!(payload:? = x; "received unexpected");
                let text = x.into_data();
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
        host_data: Arc<serde_json::Value>,
    ) -> Self {
        Self {
            imp: tokio::sync::Mutex::new(ModuleImpl { url, stream: None }),
            cancellation,
            cookie,
            name,
            host_data,
        }
    }

    pub async fn close(&self) {
        let mut lock = self.imp.lock().await;
        if let Some(stream) = &mut lock.stream {
            if stream.is_terminated() {
                return;
            }
            if let Err(e) = stream.close(None).await {
                log_error!(error:err = e; "closing stream");
            }
        }
    }

    async fn send_impl<R, V>(&self, val: V) -> anyhow::Result<std::result::Result<R, GenericValue>>
    where
        V: serde::Serialize,
        R: serde::Serialize + serde::de::DeserializeOwned,
    {
        let mut zelf = self.imp.lock().await;

        if zelf.stream.is_none() {
            log_info!(url = zelf.url, name = self.name; "initializing connection to module");

            let (mut ws_stream, _) = tokio_tungstenite::connect_async(&zelf.url).await?;

            let msg = calldata::to_value(&genvm_modules_interfaces::GenVMHello {
                cookie: self.cookie.clone(),
                host_data: self.host_data.clone(),
            })?;

            ws_stream
                .send(Message::Binary(calldata::encode(&msg).into()))
                .await?;

            zelf.stream = Some(ws_stream);
        }

        match &mut zelf.stream {
            None => unreachable!(),
            Some(stream) => {
                let val = calldata::to_value(&val)?;
                let payload = calldata::encode(&val);
                stream.send(Message::Binary(payload.into())).await?;
                let response = read_handling_pings(stream).await?;

                let response = calldata::decode(&response)?;

                log_info!(name = self.name, question:serde = val, response:? = response; "answer from module");

                let res: genvm_modules_interfaces::Result<R> =
                    calldata::from_value(response).with_context(|| "parsing result of module")?;

                match res {
                    genvm_modules_interfaces::Result::Ok(v) => Ok(Ok(v)),
                    genvm_modules_interfaces::Result::UserError(value) => Ok(Err(value)),
                    genvm_modules_interfaces::Result::FatalError(value) => {
                        log_error!(error = value; "module error");
                        Err(anyhow::anyhow!("module error: {value}"))
                    }
                }
            }
        }
    }

    pub async fn send<R, V>(&self, val: V) -> anyhow::Result<std::result::Result<R, GenericValue>>
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
