use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::{Message, Utf8Bytes};

type WSStream =
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

struct ModuleImpl {
    url: String,
    stream: Option<WSStream>,
}

pub struct Module {
    imp: tokio::sync::Mutex<ModuleImpl>,
}

async fn read_handling_pings(stream: &mut WSStream) -> anyhow::Result<Utf8Bytes> {
    loop {
        match stream
            .next()
            .await
            .ok_or(anyhow::anyhow!("service closed connection"))??
        {
            Message::Ping(v) => {
                stream.send(Message::Pong(v)).await?;
            }
            Message::Pong(_) => {}
            Message::Close(_) => anyhow::bail!("stream closed"),
            x => {
                let text = x.into_text()?;
                return Ok(text);
            }
        }
    }
}

impl Module {
    pub fn new(url: String) -> Self {
        Self {
            imp: tokio::sync::Mutex::new(ModuleImpl { url, stream: None }),
        }
    }

    pub async fn send<R, V>(
        &self,
        val: V,
    ) -> anyhow::Result<std::result::Result<R, serde_json::Value>>
    where
        V: serde::Serialize,
        R: serde::de::DeserializeOwned,
    {
        let mut zelf = self.imp.lock().await;

        if zelf.stream.is_none() {
            let (ws_stream, _) = tokio_tungstenite::connect_async(&zelf.url).await?;
            zelf.stream = Some(ws_stream);
        }

        match &mut zelf.stream {
            None => unreachable!(),
            Some(stream) => {
                let payload = serde_json::to_string(&val)?;
                stream.send(Message::Text(payload.into())).await?;
                let response = read_handling_pings(stream).await?;

                let res: genvm_modules_interfaces::Result<R> = serde_json::from_str(&response)?;
                return match res {
                    genvm_modules_interfaces::Result::Ok(v) => Ok(Ok(v)),
                    genvm_modules_interfaces::Result::UserError(value) => Ok(Err(value)),
                    genvm_modules_interfaces::Result::FatalError(value) => {
                        log::error!(error:? = value; "module error");
                        Err(anyhow::anyhow!("module error"))
                    }
                };
            }
        }
    }
}
