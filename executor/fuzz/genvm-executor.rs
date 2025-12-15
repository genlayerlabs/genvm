use std::{
    collections::HashMap,
    io::Write,
    os::fd::FromRawFd,
    sync::{atomic::AtomicBool, Arc},
};

use anyhow::Context;
use arbitrary::Arbitrary;
use genvm::public_abi;
use genvm_common::*;
use tokio::io::AsyncWriteExt;

fn wasm_smith_config() -> wasm_smith::Config {
    let mut config = wasm_smith::Config::default();
    config.simd_enabled = false;
    config.bulk_memory_enabled = true;
    config.reference_types_enabled = false;
    config.relaxed_simd_enabled = false;
    config.saturating_float_to_int_enabled = false;
    config.simd_enabled = false;
    config.threads_enabled = false;
    config.memory64_enabled = false;

    config
}

#[derive(Clone, Copy, Debug)]
struct FuzzNext;

impl std::fmt::Display for FuzzNext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "fuzz-next")
    }
}

impl std::error::Error for FuzzNext {}

fn fuzz_next() -> anyhow::Error {
    anyhow::Error::from(FuzzNext)
}

struct DuplexStream {
    file_to_read: std::fs::File,
    file_to_write: std::fs::File,
}

impl DuplexStream {
    fn into_async(self) -> AsyncDuplexStream {
        //unsafe {
        //    libc::fcntl(
        //        self.file_to_read.as_raw_fd(),
        //        libc::F_SETFL,
        //        libc::O_NONBLOCK,
        //    );
        //    libc::fcntl(
        //        self.file_to_write.as_raw_fd(),
        //        libc::F_SETFL,
        //        libc::O_NONBLOCK,
        //    );
        //}
        AsyncDuplexStream {
            file_to_read: tokio::fs::File::from_std(self.file_to_read),
            file_to_write: tokio::fs::File::from_std(self.file_to_write),
        }
    }
}

struct AsyncDuplexStream {
    file_to_read: tokio::fs::File,
    file_to_write: tokio::fs::File,
}

impl tokio::io::AsyncRead for AsyncDuplexStream {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let this = self.get_mut();
        tokio::io::AsyncRead::poll_read(std::pin::Pin::new(&mut this.file_to_read), cx, buf)
    }
}

impl tokio::io::AsyncWrite for AsyncDuplexStream {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<Result<usize, std::io::Error>> {
        let zelf = self.get_mut();
        tokio::io::AsyncWrite::poll_write(std::pin::Pin::new(&mut zelf.file_to_write), cx, buf)
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        let zelf = self.get_mut();
        tokio::io::AsyncWrite::poll_flush(std::pin::Pin::new(&mut zelf.file_to_write), cx)
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), std::io::Error>> {
        let zelf = self.get_mut();
        tokio::io::AsyncWrite::poll_shutdown(std::pin::Pin::new(&mut zelf.file_to_write), cx)
    }
}

fn connected_files() -> (std::fs::File, std::fs::File) {
    let mut fds: [i32; 2] = [0; 2];

    let code = unsafe { libc::pipe(std::ptr::from_mut(&mut fds).cast()) };
    if code != 0 {
        panic!("failed to create pipe: {}", std::io::Error::last_os_error());
    }

    let read_file = unsafe { std::fs::File::from_raw_fd(fds[0]) };
    let write_file = unsafe { std::fs::File::from_raw_fd(fds[1]) };

    (read_file, write_file)
}

impl DuplexStream {
    fn new() -> (Self, Self) {
        let (read_file_a, write_file_a) = connected_files();
        let (read_file_b, write_file_b) = connected_files();

        (
            Self {
                file_to_read: read_file_a,
                file_to_write: write_file_b,
            },
            Self {
                file_to_read: read_file_b,
                file_to_write: write_file_a,
            },
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SequentialEvent {
    Event {
        topics: Vec<[u8; 32]>,
        blob: Vec<u8>,
    },
    Message {
        to: genvm::calldata::Address,
        internal: bool,
        data: Vec<u8>,
    },
}

#[derive(Debug, PartialEq, Eq)]
struct HostAccumulatedData {
    messages: Vec<SequentialEvent>,
    eq_outputs: Vec<Vec<u8>>,
    result: Vec<u8>,
}

mod mock_host {
    use anyhow::Result;
    use genvm::host::host_fns;
    use genvm::public_abi::StorageType;
    use genvm_common::log_debug;
    use std::collections::HashMap;
    use std::io::{Read, Write};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    use crate::{HostAccumulatedData, SequentialEvent};

    use super::AsyncDuplexStream;

    const ACCOUNT_ADDR_SIZE: usize = 20;
    const SLOT_ID_SIZE: usize = 32;

    pub struct MockWriter {
        pub sock: AsyncDuplexStream,
    }

    pub struct MockHost {
        pub sock: MockWriter,
        pub eq_outputs: Option<Vec<Vec<u8>>>,
        pub storage: HashMap<[u8; 36], [u8; 32]>,
        pub calldata: Vec<u8>,
        pub collected_eq_outputs: Vec<Vec<u8>>,

        pub seq_events: Vec<SequentialEvent>,
    }

    impl MockWriter {
        async fn read_u32(&mut self) -> Result<u32> {
            let mut buf = [0u8; 4];
            self.sock.read_exact(&mut buf).await?;
            Ok(u32::from_le_bytes(buf))
        }

        async fn read_u64(&mut self) -> Result<u64> {
            let mut buf = [0u8; 8];
            self.sock.read_exact(&mut buf).await?;
            Ok(u64::from_le_bytes(buf))
        }

        async fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
            self.sock.read_exact(buf).await?;
            Ok(())
        }

        async fn read_slice(&mut self) -> Result<Vec<u8>> {
            let len = self.read_u32().await?;
            let mut buf = vec![0u8; len as usize];
            self.read_exact(&mut buf).await?;
            Ok(buf)
        }

        async fn write_u32(&mut self, value: u32) -> Result<()> {
            self.sock.write_all(&value.to_le_bytes()).await?;
            Ok(())
        }

        async fn write_u64(&mut self, value: u64) -> Result<()> {
            self.sock.write_all(&value.to_le_bytes()).await?;
            Ok(())
        }

        async fn write_slice(&mut self, data: &[u8]) -> Result<()> {
            self.write_u32(data.len() as u32).await?;
            self.sock.write_all(data).await?;
            Ok(())
        }

        async fn write_error(&mut self, error: host_fns::Errors) -> Result<()> {
            self.sock.write_all(&[error as u8]).await?;
            Ok(())
        }
    }

    impl MockHost {
        pub fn write_storage_slice(
            &mut self,
            slot_id: [u8; 32],
            index: u32,
            data: &[u8],
        ) -> anyhow::Result<()> {
            let mut remaining_data = data;
            let mut current_index = index;

            while !remaining_data.is_empty() {
                let page_index = current_index / 32;
                let offset_in_page = current_index % 32;
                let bytes_to_write =
                    std::cmp::min(32 - offset_in_page as usize, remaining_data.len());

                // Create page key: slot_id + page_index
                let mut page_key = [0u8; 36];
                page_key[..32].copy_from_slice(&slot_id);
                page_key[32..36].copy_from_slice(&page_index.to_le_bytes());

                // Get existing page or create new zero-filled page
                let mut page = self.storage.get(&page_key).copied().unwrap_or([0u8; 32]);

                // Write data to the correct offset within the page
                let start_offset = offset_in_page as usize;
                let end_offset = start_offset + bytes_to_write;
                page[start_offset..end_offset].copy_from_slice(&remaining_data[..bytes_to_write]);

                // Store the modified page
                self.storage.insert(page_key, page);

                // Move to next chunk
                remaining_data = &remaining_data[bytes_to_write..];
                current_index += bytes_to_write as u32;
            }

            Ok(())
        }

        pub fn read_storage_slice(
            &mut self,
            slot_id: [u8; 32],
            index: u32,
            data: &mut [u8],
        ) -> anyhow::Result<()> {
            let mut remaining_data = data;
            let mut current_index = index;

            while !remaining_data.is_empty() {
                let page_index = current_index / 32;
                let offset_in_page = current_index % 32;
                let bytes_to_read =
                    std::cmp::min(32 - offset_in_page as usize, remaining_data.len());

                // Create page key: slot_id + page_index
                let mut page_key = [0u8; 36];
                page_key[..32].copy_from_slice(&slot_id);
                page_key[32..36].copy_from_slice(&page_index.to_le_bytes());

                // Get page or use zero-filled page if not found
                let page = self.storage.get(&page_key).copied().unwrap_or([0u8; 32]);

                // Read data from the correct offset within the page
                let start_offset = offset_in_page as usize;
                let end_offset = start_offset + bytes_to_read;
                remaining_data[..bytes_to_read].copy_from_slice(&page[start_offset..end_offset]);

                // Move to next chunk
                remaining_data = &mut remaining_data[bytes_to_read..];
                current_index += bytes_to_read as u32;
            }

            Ok(())
        }

        pub fn write_code(&mut self, code: &[u8]) -> anyhow::Result<()> {
            let code_slot =
                genvm::SlotID::ZERO.indirection(genvm::host::message::root_offsets::CODE);

            let code_size = u32::to_le_bytes(code.len() as u32);
            self.write_storage_slice(code_slot.0, 0, &code_size)?;
            self.write_storage_slice(code_slot.0, 4, code)?;

            Ok(())
        }

        pub async fn run(self) -> anyhow::Result<HostAccumulatedData> {
            let res = self.run_impl().await;

            log_debug!(res:? = res; "mock host finished");

            res
        }

        async fn run_impl(mut self) -> anyhow::Result<HostAccumulatedData> {
            loop {
                let mut method_buf = [0u8; 1];
                self.sock.read_exact(&mut method_buf).await?;

                let method = host_fns::Methods::try_from(method_buf[0])
                    .map_err(|_| anyhow::anyhow!("Unknown method: {}", method_buf[0]))?;

                log_debug!(method:? = method; "mock host called");

                match method {
                    host_fns::Methods::GetCalldata => {
                        self.sock.write_error(host_fns::Errors::Ok).await?;
                        self.sock.write_slice(&self.calldata).await?;
                        self.sock.sock.flush().await?;
                    }

                    host_fns::Methods::StorageRead => {
                        let mut mode_buf = [0u8; 1];
                        self.sock.read_exact(&mut mode_buf).await?;
                        let _mode = StorageType::try_from(mode_buf[0])
                            .map_err(|_| anyhow::anyhow!("Invalid storage type"))?;

                        let mut account = [0u8; ACCOUNT_ADDR_SIZE];
                        self.sock.read_exact(&mut account).await?;

                        let mut slot = [0u8; SLOT_ID_SIZE];
                        self.sock.read_exact(&mut slot).await?;

                        let index = self.sock.read_u32().await?;
                        let len = self.sock.read_u32().await?;

                        let mut data = vec![0u8; len as usize];
                        self.read_storage_slice(slot, index, &mut data)?;

                        self.sock.write_error(host_fns::Errors::Ok).await?;
                        self.sock.sock.write_all(&data).await?;
                        self.sock.sock.flush().await?;
                    }

                    host_fns::Methods::StorageWrite => {
                        let mut slot = [0u8; SLOT_ID_SIZE];
                        self.sock.read_exact(&mut slot).await?;

                        let index = self.sock.read_u32().await?;
                        let len = self.sock.read_u32().await?;

                        let mut data = vec![0u8; len as usize];
                        self.sock.read_exact(&mut data).await?;

                        self.write_storage_slice(slot, index, &data)?;

                        self.sock.write_error(host_fns::Errors::Ok).await?;
                        self.sock.sock.flush().await?;
                    }

                    host_fns::Methods::ConsumeResult => {
                        let result = self.sock.read_slice().await?;
                        // Write acknowledgment
                        self.sock.sock.write_all(&[0x00]).await?;
                        self.sock.sock.flush().await?;

                        return Ok(HostAccumulatedData {
                            messages: self.seq_events,
                            eq_outputs: self.collected_eq_outputs,
                            result,
                        });
                    }

                    host_fns::Methods::GetLeaderNondetResult => {
                        let call_no = self.sock.read_u32().await?;

                        if let Some(ref eq_outputs) = self.eq_outputs {
                            if call_no < eq_outputs.len() as u32 {
                                let output = &eq_outputs[call_no as usize];
                                self.sock.write_error(host_fns::Errors::Ok).await?;
                                self.sock.write_slice(output).await?;
                            } else {
                                self.sock.write_error(host_fns::Errors::Absent).await?;
                            }
                        } else {
                            self.sock.write_error(host_fns::Errors::IAmLeader).await?;
                        }
                        self.sock.sock.flush().await?;
                    }

                    host_fns::Methods::PostNondetResult => {
                        let call_no = self.sock.read_u32().await?;
                        let result = self.sock.read_slice().await?;

                        while self.collected_eq_outputs.len() <= call_no as usize {
                            self.collected_eq_outputs.push(Vec::new());
                        }

                        // Store the result for later comparison
                        self.collected_eq_outputs[call_no as usize] = result;

                        self.sock.write_error(host_fns::Errors::Ok).await?;
                        self.sock.sock.flush().await?;
                    }

                    host_fns::Methods::PostMessage => {
                        let mut account = [0u8; ACCOUNT_ADDR_SIZE];
                        self.sock.read_exact(&mut account).await?;

                        let calldata = self.sock.read_slice().await?;
                        let message_data = self.sock.read_slice().await?;

                        self.seq_events.push(SequentialEvent::Message {
                            to: genvm::calldata::Address::from(account),
                            internal: true,
                            data: calldata,
                        });

                        self.sock.write_error(host_fns::Errors::Ok).await?;
                        self.sock.sock.flush().await?;
                    }

                    host_fns::Methods::ConsumeFuel => {
                        let _gas = self.sock.read_u64().await?;
                        // No response needed for consume_fuel
                    }

                    host_fns::Methods::DeployContract => {
                        let mut calldata = self.sock.read_slice().await?;
                        let code = self.sock.read_slice().await?;
                        let message_data = self.sock.read_slice().await?;

                        calldata.extend_from_slice(&code); // just ok

                        self.seq_events.push(SequentialEvent::Message {
                            to: genvm::calldata::Address::zero(),
                            internal: true,
                            data: calldata,
                        });

                        self.sock.write_error(host_fns::Errors::Ok).await?;
                        self.sock.sock.flush().await?;
                    }

                    host_fns::Methods::EthCall => {
                        let mut account = [0u8; ACCOUNT_ADDR_SIZE];
                        self.sock.read_exact(&mut account).await?;

                        let _calldata = self.sock.read_slice().await?;

                        // Return empty result for mock
                        self.sock.write_error(host_fns::Errors::Ok).await?;
                        self.sock.write_slice(&[]).await?;
                        self.sock.sock.flush().await?;
                    }

                    host_fns::Methods::EthSend => {
                        let mut account = [0u8; ACCOUNT_ADDR_SIZE];
                        self.sock.read_exact(&mut account).await?;

                        let calldata = self.sock.read_slice().await?;
                        let message_data = self.sock.read_slice().await?;

                        self.seq_events.push(SequentialEvent::Message {
                            to: genvm::calldata::Address::from(account),
                            internal: false,
                            data: calldata,
                        });

                        self.sock.write_error(host_fns::Errors::Ok).await?;
                        self.sock.sock.flush().await?;
                    }

                    host_fns::Methods::PostEvent => {
                        let topics_len = {
                            let mut buf = [0u8; 1];
                            self.sock.read_exact(&mut buf).await?;
                            buf[0]
                        };

                        let mut topics = Vec::new();

                        for _ in 0..topics_len {
                            let mut topic = [0u8; 32];
                            self.sock.read_exact(&mut topic).await?;

                            topics.push(topic);
                        }

                        let blob = self.sock.read_slice().await?;

                        self.seq_events
                            .push(SequentialEvent::Event { topics, blob });

                        self.sock.write_error(host_fns::Errors::Ok).await?;
                        self.sock.sock.flush().await?;
                    }

                    host_fns::Methods::GetBalance => {
                        let mut account = [0u8; ACCOUNT_ADDR_SIZE];
                        self.sock.read_exact(&mut account).await?;

                        // Return zero balance
                        self.sock.write_error(host_fns::Errors::Ok).await?;
                        let balance = [0u8; 32]; // 256-bit zero
                        self.sock.sock.write_all(&balance).await?;
                        self.sock.sock.flush().await?;
                    }

                    host_fns::Methods::RemainingFuelAsGen => {
                        // Return large fuel amount
                        self.sock.write_error(host_fns::Errors::Ok).await?;
                        self.sock.write_u64(1u64 << 50).await?;
                        self.sock.sock.flush().await?;
                    }

                    host_fns::Methods::NotifyNondetDisagreement => {
                        let _call_no = self.sock.read_u32().await?;

                        anyhow::bail!("should not happen");
                    }
                }
            }
        }
    }
}

impl std::io::Read for DuplexStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.file_to_read.read(buf)
    }
}

impl std::io::Write for DuplexStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.file_to_write.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.file_to_write.flush()
    }
}

impl genvm::host::Sock for DuplexStream {}

fn generate_contract(data: &FuzzingInput) -> anyhow::Result<Vec<u8>> {
    let mut config_a = wasm_smith_config();
    config_a.available_imports = Some(wat::parse_str(include_str!(
        "genvm-executor/a-imports.wat"
    ))?);
    config_a.exports = Some(wat::parse_str(include_str!(
        "genvm-executor/a-exports.wat"
    ))?);

    let mut config_b = wasm_smith_config();
    config_b.available_imports = Some(wat::parse_str(include_str!(
        "genvm-executor/b-imports.wat"
    ))?);
    config_b.exports = Some(wat::parse_str(include_str!(
        "genvm-executor/b-exports.wat"
    ))?);

    let module_a = wasm_smith::Module::new(
        config_a,
        &mut arbitrary::Unstructured::new(&data.wasm_a_data),
    )?;
    let module_b = wasm_smith::Module::new(
        config_b,
        &mut arbitrary::Unstructured::new(&data.wasm_b_data),
    )?;

    let mut module_a = module_a.encoded();
    let mut name_section = wasm_encoder::NameSection::new();
    name_section.module("mod_a");
    module_a.section(&name_section);
    let module_a_bytes = module_a.finish();

    let module_b_bytes = module_b.to_bytes();

    let mut runner_zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));

    let fopts = zip::write::FileOptions::<'_, ()>::default()
        .compression_method(zip::CompressionMethod::Stored);

    runner_zip.start_file("module_a.wasm", fopts)?;
    runner_zip.write_all(&module_a_bytes)?;
    runner_zip.start_file("module_b.wasm", fopts)?;
    runner_zip.write_all(&module_b_bytes)?;
    runner_zip.start_file("runner.json", fopts)?;
    runner_zip.write_all(
        r#"
        {
            "Seq": [
                { "LinkWasm": "module_a.wasm" },
                { "StartWasm": "module_b.wasm" }
            ]
        }
    "#
        .as_bytes(),
    )?;
    let contract_buf = runner_zip.finish()?.into_inner();

    Ok(contract_buf)
}

#[derive(Debug)]
struct ReturnToCompare {
    kind: public_abi::ResultCode,
    result_data: calldata::Value,
    fingerprint: Option<genvm::rt::errors::Fingerprint>,
}

#[derive(Debug)]
struct LeaderData {
    retn: ReturnToCompare,
    host_data: HostAccumulatedData,
}

async fn start_timeouts(
    test_done: Arc<cancellation::Token>,
) -> (Arc<cancellation::Token>, Arc<AtomicBool>) {
    let cancelled = Arc::new(AtomicBool::new(false));
    let (token, canceller) = genvm_common::cancellation::make();

    let c = cancelled.clone();

    tokio::spawn(async move {
        tokio::select! {
            _ = test_done.chan.closed() => {
            }
            _ = tokio::time::sleep(std::time::Duration::from_secs(20)) => {
                c.store(true, std::sync::atomic::Ordering::SeqCst);
                canceller();
            }
        }
    });

    (token, cancelled)
}

async fn run_with(
    data: &FuzzingInput,
    contract_code: &[u8],
    eq_outputs: Option<Vec<Vec<u8>>>,
    test_done: Arc<cancellation::Token>,
) -> anyhow::Result<LeaderData> {
    let (token, had_timeout) = start_timeouts(test_done).await;

    let shared_data = sync::DArc::new(genvm::rt::SharedData {
        cancellation: token,
        is_sync: eq_outputs.is_some(),
        genvm_id: genvm_modules_interfaces::GenVMId(1),
        debug_mode: false,
        metrics: genvm::Metrics::default(),
        storage_pages_limit: std::sync::atomic::AtomicU64::new(128),
    });

    let mut registry_dir = std::env::current_dir()?;
    registry_dir.push("fuzz");
    registry_dir.push("genvm-executor");
    registry_dir.push("registry");

    let (duplex_a, duplex_b) = DuplexStream::new();

    let config = genvm::config::Config {
        modules: genvm::config::Modules {
            llm: genvm::config::Module {
                address: "".to_owned(),
            },
            web: genvm::config::Module {
                address: "".to_owned(),
            },
        },
        cache_dir: "/dev/null".to_owned(),
        runners_dir: "/tmp".to_owned(), // we have no runners
        registry_dir: registry_dir.to_string_lossy().to_string(),
        base: BaseConfig {
            threads: 2,
            blocking_threads: 4,
            log_level: logger::Level::Info,
            log_disable: "".to_owned(),
        },
    };

    let host_data = genvm_modules_interfaces::HostData {
        node_address: "0x".to_owned(),
        tx_id: "0x".to_owned(),
        rest: Default::default(),
    };

    let host = genvm::Host::new(Box::new(duplex_b), shared_data.gep(|x| &x.metrics.host));

    let mut actual_host = mock_host::MockHost {
        eq_outputs,
        sock: mock_host::MockWriter {
            sock: duplex_a.into_async(),
        },
        storage: HashMap::new(),
        calldata: Vec::new(),
        collected_eq_outputs: Vec::new(),
        seq_events: Vec::new(),
    };

    actual_host.write_code(contract_code)?;

    let actual_host_future = tokio::spawn(actual_host.run());

    let supervisor = genvm::create_supervisor(&config, host, host_data, shared_data, &data.msg)
        .with_context(|| "creating supervisor")?;

    let (full_res, _) = genvm::run_with(data.msg.clone(), supervisor, &data.get_perms())
        .await
        .with_context(|| "running")?;

    let host_data = actual_host_future.await??;

    if had_timeout.load(std::sync::atomic::Ordering::SeqCst) {
        return Err(fuzz_next());
    }

    Ok(LeaderData {
        retn: ReturnToCompare {
            kind: full_res.kind,
            result_data: full_res.data,
            fingerprint: full_res.fingerprint,
        },
        host_data,
    })
}

struct CallOnDrop<F: FnOnce() + Clone>(Option<F>);

impl<F: FnOnce() + Clone> Drop for CallOnDrop<F> {
    fn drop(&mut self) {
        if let Some(foo) = self.0.take() {
            foo();
        }
    }
}

async fn do_fuzzing(data: FuzzingInput, contract_code: &[u8]) -> anyhow::Result<()> {
    let (token, canceller) = genvm_common::cancellation::make();

    let _ = CallOnDrop(Some(canceller));

    let leader_res = run_with(&data, contract_code, None, token.clone())
        .await
        .with_context(|| "as leader")?;

    log_debug!(leader_res:? = leader_res; "leader result");

    let validator_res = run_with(
        &data,
        contract_code,
        Some(leader_res.host_data.eq_outputs),
        token.clone(),
    )
    .await
    .with_context(|| "as sync")?;

    log_debug!(validator_res:? = validator_res; "validator result");

    assert_eq!(leader_res.retn.fingerprint, validator_res.retn.fingerprint);
    assert_eq!(
        leader_res.host_data.messages,
        validator_res.host_data.messages
    );

    assert_eq!(leader_res.retn.kind, validator_res.retn.kind);
    assert_eq!(leader_res.retn.result_data, validator_res.retn.result_data);

    assert_eq!(leader_res.host_data.result, validator_res.host_data.result);

    log_debug!("OK!");

    Ok(())
}

fn run(data: FuzzingInput) -> anyhow::Result<()> {
    let contract_code = generate_contract(&data)?;

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(2)
        .max_blocking_threads(4)
        .build()?;

    rt.block_on(do_fuzzing(data, &contract_code))?;

    log_debug!("dropping rt");
    std::mem::drop(rt);
    log_debug!("rt dropped");

    Ok(())
}

#[derive(Debug, Clone, Arbitrary)]
struct FuzzingInput {
    wasm_a_data: Vec<u8>,
    wasm_b_data: Vec<u8>,
    msg: genvm::MessageData,
    can_write: bool, // wscn
    can_send: bool,
    can_nondet: bool,
}

impl FuzzingInput {
    fn get_perms(&self) -> String {
        let mut perms = "r".to_owned();

        if self.can_write {
            perms.push('w');
        }
        if self.can_send {
            perms.push('s');
        }
        if self.can_nondet {
            perms.push('n');
        }
        perms
    }
}

fn main() {
    genvm_common::logger::initialize(
        logger::Level::Debug,
        "genvm::rt::memlimiter*",
        std::io::stderr(),
    );

    afl::fuzz!(|data: FuzzingInput| {
        let res = run(data);

        if let Err(err) = &res {
            if err.downcast_ref::<FuzzNext>().is_none() {
                eprintln!("error: {err:?}");
                panic!("error detected!!!");
            } else {
                log_info!(error:ah = err; "fuzz next");
            }
        }
    });

    log_info!("fuzzing done");
}
