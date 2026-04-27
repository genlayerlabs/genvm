use genvm_common::io::Stream;
use genvm_common::*;
use std::sync::Arc;

use anyhow::Context as _;
use genvm_common::calldata;
use genvm_modules_interfaces::GenericValue;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Boxed stream type for module connections
type BoxedStream = Box<dyn Stream>;

struct ModuleImpl {
    url: String,
    stream: Option<BoxedStream>,
}

pub struct Module {
    name: String,
    cancellation: Arc<genvm_common::cancellation::Token>,
    imp: tokio::sync::Mutex<ModuleImpl>,
    genvm_id: genvm_modules_interfaces::GenVMId,
    host_data: genvm_modules_interfaces::HostData,
    metrics: sync::DArc<Metrics>,
}

#[derive(Default, Debug, serde::Serialize, genlayer_calldata::Encode)]
pub struct Metrics {
    pub calls: genvm_common::stats::metric::Count,
    pub time: stats::metric::Time,
}

/// Write a length-prefixed message to a stream
/// Format: 4 bytes big-endian length + body
async fn write_message(stream: &mut BoxedStream, data: &[u8]) -> anyhow::Result<()> {
    let len = data.len() as u32;
    stream.write_all(&len.to_be_bytes()).await?;
    stream.write_all(data).await?;
    stream.flush().await?;
    Ok(())
}

/// Read a length-prefixed message from a stream
/// Format: 4 bytes big-endian length + body
async fn read_message(stream: &mut BoxedStream) -> anyhow::Result<Vec<u8>> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;

    let mut data = vec![0u8; len];
    stream.read_exact(&mut data).await?;
    Ok(data)
}

const UNIX_PREFIX: &str = "unix://";
const FD_PREFIX: &str = "fd://";

/// Connect to a module endpoint
/// Supports:
/// - TCP: "host:port" or "ws://host:port" or "wss://host:port"
/// - Unix socket: "unix:///path/to/socket"
/// - File descriptor: "fd://fd" (for bidirectional fd like socketpair)
async fn connect(url: &str) -> anyhow::Result<BoxedStream> {
    if let Some(socket_path) = url.strip_prefix(UNIX_PREFIX) {
        let stream = tokio::net::UnixStream::connect(socket_path)
            .await
            .with_context(|| format!("connecting to unix socket {socket_path}"))?;
        Ok(Box::new(stream))
    } else if let Some(fd_str) = url.strip_prefix(FD_PREFIX) {
        let fd: i32 = fd_str
            .parse()
            .with_context(|| format!("parsing fd from '{}'", fd_str))?;

        let stream = unsafe { genvm_common::io::AsyncCustomFD::from_raw_fd(fd)? };
        Ok(Box::new(stream))
    } else {
        // Strip ws:// or wss:// prefix if present for TCP connection
        let addr = url
            .strip_prefix("ws://")
            .or_else(|| url.strip_prefix("wss://"))
            .unwrap_or(url);

        let stream = tokio::net::TcpStream::connect(addr)
            .await
            .with_context(|| format!("connecting to TCP {addr}"))?;
        Ok(Box::new(stream))
    }
}

impl Module {
    pub fn new(
        name: String,
        url: String,
        cancellation: Arc<genvm_common::cancellation::Token>,
        genvm_id: genvm_modules_interfaces::GenVMId,
        host_data: genvm_modules_interfaces::HostData,
        metrics: sync::DArc<Metrics>,
    ) -> Self {
        Self {
            imp: tokio::sync::Mutex::new(ModuleImpl { url, stream: None }),
            cancellation,
            genvm_id,
            name,
            host_data,
            metrics,
        }
    }

    pub async fn close(&self) {
        let mut lock = self.imp.lock().await;
        if let Some(stream) = &mut lock.stream {
            if let Err(e) = stream.shutdown().await {
                log_error!(error:err = e; "closing stream");
            }
        }
        lock.stream = None;
    }

    async fn send_impl<R, V>(&self, val: V) -> anyhow::Result<std::result::Result<R, GenericValue>>
    where
        V: calldata::codec::Encode<Vec<u8>, Error = std::convert::Infallible>,
        R: calldata::codec::Decode,
    {
        self.metrics.calls.increment();

        let zelf = self.imp.lock().await;

        let mut zelf = sync::Lock::new(
            zelf,
            stats::tracker::Time::new(self.metrics.gep(|x| &x.time)),
        );

        if zelf.stream.is_none() {
            log_debug!(url = zelf.url, name = self.name; "initializing connection to module");

            let mut stream = connect(&zelf.url)
                .await
                .with_context(|| format!("connecting to {}", zelf.url))?;

            write_message(
                &mut stream,
                &calldata::encode_obj(&genvm_modules_interfaces::GenVMHello {
                    genvm_id: self.genvm_id,
                    host_data: self.host_data.clone(),
                }),
            )
            .await?;

            log_debug!(name = self.name; "connection to module initialized");

            zelf.stream = Some(stream);
        }

        match &mut zelf.stream {
            None => unreachable!(),
            Some(stream) => {
                let val = calldata::to_value(&val);
                let payload = calldata::encode(&val);
                write_message(stream, &payload).await?;
                let response = read_message(stream).await?;

                let response = calldata::decode(&response)?;

                log_info!(name = self.name, question:serde = val, response:serde = response; "answer from module");

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

    pub async fn get_stats<V>(&self, val: V) -> anyhow::Result<calldata::Value>
    where
        V: calldata::codec::Encode<Vec<u8>, Error = std::convert::Infallible>,
    {
        let zelf = self.imp.lock().await;
        let mut zelf = sync::Lock::new(zelf, self.metrics.gep(|x| &x.time));

        match &mut zelf.stream {
            None => Ok(calldata::Value::Null),
            Some(stream) => {
                write_message(stream, &calldata::encode_obj(&val)).await?;
                let response = read_message(stream).await?;

                let response = calldata::decode(&response)?;

                Ok(response)
            }
        }
    }

    pub async fn send<R, V>(&self, val: V) -> anyhow::Result<std::result::Result<R, GenericValue>>
    where
        V: calldata::codec::Encode<Vec<u8>, Error = std::convert::Infallible>,
        R: calldata::codec::Decode,
    {
        tokio::select! {
            _ = self.cancellation.chan.closed() => {
                anyhow::bail!("timeout") // it will be replaced later
            }
            res = self.send_impl(val) => {
                res.with_context(|| "sending request to module")
            }
        }
    }
}

pub struct All {
    pub web: Arc<Module>,
    pub llm: Arc<Module>,
}
