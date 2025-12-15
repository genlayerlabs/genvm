use std::collections::BTreeMap;
use std::sync::Arc;

use genvm_common::*;

use genvm_modules_interfaces::GenericValue;
use wiggle::GuestError;

use crate::host::{self, SlotID};
use crate::{calldata, public_abi, rt};

use super::{base, gl_call, vfs};

fn entry_kind_as_int<S>(data: &public_abi::EntryKind, d: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    d.serialize_u8(*data as u8)
}

#[derive(serde::Serialize, Debug)]
pub struct ExtendedMessage {
    pub contract_address: calldata::Address,
    pub sender_address: calldata::Address,
    pub origin_address: calldata::Address,
    /// View methods call chain.
    /// It is empty for entrypoint (refer to [`contract_address`])
    pub stack: Vec<calldata::Address>,

    pub chain_id: num_bigint::BigInt,
    pub value: num_bigint::BigInt,
    pub is_init: bool,
    /// Transaction timestamp
    pub datetime: chrono::DateTime<chrono::Utc>,

    #[serde(serialize_with = "entry_kind_as_int")]
    pub entry_kind: public_abi::EntryKind,
    #[serde(with = "serde_bytes")]
    pub entry_data: Vec<u8>,

    pub entry_stage_data: calldata::Value,
}

fn default_entry_stage_data() -> calldata::Value {
    calldata::Value::Null
}

impl ExtendedMessage {
    pub fn fork_leader(
        &self,
        entry_kind: public_abi::EntryKind,
        entry_data: Vec<u8>,
        entry_leader_data: Option<rt::vm::RunOk>,
    ) -> Self {
        let entry_leader_data = match entry_leader_data {
            None => default_entry_stage_data(),
            Some(entry_leader_data) => calldata::Value::Map(BTreeMap::from([(
                "leaders_result".into(),
                calldata::Value::Bytes(Vec::from_iter(entry_leader_data.as_bytes_iter())),
            )])),
        };

        ExtendedMessage {
            contract_address: self.contract_address,
            sender_address: self.sender_address,
            origin_address: self.origin_address,
            stack: self.stack.clone(),
            chain_id: self.chain_id.clone(),
            value: self.value.clone(),
            is_init: false,
            datetime: self.datetime,
            entry_kind,
            entry_data,
            entry_stage_data: entry_leader_data,
        }
    }

    pub fn fork(&self, entry_kind: public_abi::EntryKind, entry_data: Vec<u8>) -> Self {
        self.fork_leader(entry_kind, entry_data, None)
    }
}

#[derive(Clone)]
pub struct ReadToken {
    pub mode: public_abi::StorageType,
    pub account: calldata::Address,
}

pub struct StorageHostLock<'a>(tokio::sync::MutexGuard<'a, host::Host>, ReadToken);

impl rt::vm::storage::HostStorage for StorageHostLock<'_> {
    fn storage_read(&mut self, slot_id: SlotID, index: u32, buf: &mut [u8]) -> anyhow::Result<()> {
        self.0
            .storage_read(self.1.mode, self.1.account, slot_id, index, buf)
    }
}

#[derive(Clone)]
pub struct StorageHostHolder(pub Arc<tokio::sync::Mutex<host::Host>>, pub ReadToken);

impl rt::vm::storage::HostStorageLocking for StorageHostHolder {
    type ReturnType<'a> = StorageHostLock<'a>;

    async fn lock(&self) -> Self::ReturnType<'_> {
        StorageHostLock(self.0.lock().await, self.1.clone())
    }
}

pub struct SingleVMData {
    pub conf: base::Config,
    pub message_data: ExtendedMessage,
    pub supervisor: Arc<rt::supervisor::Supervisor>,
    pub storage: rt::vm::storage::Storage<StorageHostHolder>,
    pub version: genvm_common::version::Version,
    pub should_capture_fp: Arc<std::sync::atomic::AtomicBool>,
}

pub struct Context {
    pub data: SingleVMData,
    pub messages_decremented: primitive_types::U256,

    pub start_time: std::time::Instant,
    pub prev_time: std::time::Instant,
}

pub struct ContextVFS<'a> {
    pub(super) vfs: &'a mut vfs::VFS,
    pub(super) context: &'a mut Context,
}

#[allow(clippy::too_many_arguments)]
pub(crate) mod generated {
    wiggle::from_witx!({
        witx: ["$CARGO_MANIFEST_DIR/src/wasi/witx/genlayer_sdk.witx"],
        errors: { errno => trappable Error },
        wasmtime: false,
        tracing: false,

        async: {
            genlayer_sdk::{
                gl_call,
                storage_read, storage_write,
                get_balance, get_self_balance,
            }
        },
    });

    wiggle::wasmtime_integration!({
        witx: ["$CARGO_MANIFEST_DIR/src/wasi/witx/genlayer_sdk.witx"],
        errors: { errno => trappable Error },
        target: self,
        tracing: false,

        async: {
            genlayer_sdk::{
                gl_call,
                storage_read, storage_write,
                get_balance, get_self_balance,
            }
        },
    });
}

fn read_addr_from_mem(
    mem: &mut wiggle::GuestMemory<'_>,
    addr: wiggle::GuestPtr<u8>,
) -> Result<calldata::Address, generated::types::Error> {
    let cow = mem.as_cow(addr.as_array(calldata::ADDRESS_SIZE.try_into().unwrap()))?;
    let mut ret = calldata::Address::zero();
    for (x, y) in ret.ref_mut().iter_mut().zip(cow.iter()) {
        *x = *y;
    }
    Ok(ret)
}

impl SlotID {
    fn read_from_mem(
        mem: &mut wiggle::GuestMemory<'_>,
        addr: wiggle::GuestPtr<u8>,
    ) -> Result<Self, generated::types::Error> {
        let cow = mem.as_cow(addr.as_array(SlotID::len().try_into().unwrap()))?;
        let mut ret = SlotID::zero();
        for (x, y) in ret.0.iter_mut().zip(cow.iter()) {
            *x = *y;
        }
        Ok(ret)
    }
}

fn read_owned_vec(
    mem: &mut wiggle::GuestMemory<'_>,
    ptr: wiggle::GuestPtr<[u8]>,
) -> Result<Vec<u8>, generated::types::Error> {
    Ok(mem.as_cow(ptr)?.into_owned())
}

impl Context {
    pub fn new(data: SingleVMData) -> Self {
        let now = std::time::Instant::now();

        Self {
            data,
            messages_decremented: primitive_types::U256::zero(),
            start_time: now,
            prev_time: now,
        }
    }
}

impl wiggle::GuestErrorType for generated::types::Errno {
    fn success() -> Self {
        Self::Success
    }
}

pub trait AddToLinkerFn<T> {
    fn call<'a>(&self, arg: &'a mut T) -> ContextVFS<'a>;
}

pub(super) fn add_to_linker_sync<T: Send + 'static, F>(
    linker: &mut wasmtime::Linker<T>,
    f: F,
) -> anyhow::Result<()>
where
    F: AddToLinkerFn<T> + Copy + Send + Sync + 'static,
{
    #[derive(Clone, Copy)]
    struct Fwd<F>(F);

    impl<T, F> generated::AddGenlayerSdkToLinkerFn<T> for Fwd<F>
    where
        F: AddToLinkerFn<T> + Copy + Send + Sync + 'static,
    {
        fn call(&self, arg: &mut T) -> impl generated::genlayer_sdk::GenlayerSdk {
            self.0.call(arg)
        }
    }
    generated::add_genlayer_sdk_to_linker(linker, Fwd(f))?;
    Ok(())
}

#[derive(Debug)]
pub struct ContractReturn(pub Vec<u8>);

impl std::error::Error for ContractReturn {}

impl std::fmt::Display for ContractReturn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Returned {:?}", self.0)
    }
}

impl From<GuestError> for generated::types::Error {
    fn from(err: GuestError) -> Self {
        use wiggle::GuestError::*;
        match err {
            InvalidFlagValue { .. } => generated::types::Errno::Inval.into(),
            InvalidEnumValue { .. } => generated::types::Errno::Inval.into(),
            // As per
            // https://github.com/WebAssembly/wasi/blob/main/legacy/tools/witx-docs.md#pointers
            //
            // > If a misaligned pointer is passed to a function, the function
            // > shall trap.
            // >
            // > If an out-of-bounds pointer is passed to a function and the
            // > function needs to dereference it, the function shall trap.
            //
            // so this turns OOB and misalignment errors into traps.
            PtrOverflow | PtrOutOfBounds { .. } | PtrNotAligned { .. } => {
                generated::types::Error::trap(err.into())
            }
            PtrBorrowed { .. } => generated::types::Errno::Fault.into(),
            InvalidUtf8 { .. } => generated::types::Errno::Ilseq.into(),
            TryFromIntError { .. } => generated::types::Errno::Overflow.into(),
            SliceLengthsDiffer => generated::types::Errno::Fault.into(),
            BorrowCheckerOutOfHandles => generated::types::Errno::Fault.into(),
            InFunc { err, .. } => generated::types::Error::from(*err),
        }
    }
}

impl From<std::num::TryFromIntError> for generated::types::Error {
    fn from(_err: std::num::TryFromIntError) -> Self {
        generated::types::Errno::Overflow.into()
    }
}

impl From<serde_json::Error> for generated::types::Error {
    fn from(err: serde_json::Error) -> Self {
        log_info!(error:err = err; "deserialization failed, returning inval");

        generated::types::Errno::Inval.into()
    }
}

impl ContextVFS<'_> {
    fn set_vm_run_result(
        &mut self,
        data: rt::vm::RunOk,
    ) -> Result<(generated::types::Fd, usize), generated::types::Error> {
        let data = match data {
            rt::vm::RunOk::VMError(e, cause) => {
                return Err(generated::types::Error::trap(
                    rt::errors::VMError(e, cause).into(),
                ))
            }
            data => data,
        };
        let data: Box<[u8]> = data.as_bytes_iter().collect();
        let len = data.len();
        Ok((
            generated::types::Fd::from(
                self.vfs
                    .place_content(vfs::FileContents {
                        contents: util::SharedBytes::new(data),
                        pos: 0,
                        release_memory: true,
                    })
                    .map_err(generated::types::Error::trap)?,
            ),
            len,
        ))
    }
}

async fn taskify<T>(
    fut: impl std::future::Future<Output = anyhow::Result<std::result::Result<T, GenericValue>>>
        + Send
        + 'static,
) -> anyhow::Result<Box<[u8]>>
where
    T: serde::Serialize + Send,
{
    match fut.await? {
        Ok(r) => {
            let r = calldata::to_value(&r)?;
            let data = calldata::Value::Map(BTreeMap::from([("ok".to_owned(), r)]));

            Ok(Box::from(calldata::encode(&data)))
        }
        Err(e) => {
            let e = calldata::to_value(&e)?;
            let data = calldata::Value::Map(BTreeMap::from([("error".to_owned(), e)]));

            Ok(Box::from(calldata::encode(&data)))
        }
    }
}

const NO_FILE: u32 = u32::MAX;

#[inline]
fn file_fd_none() -> generated::types::Fd {
    generated::types::Fd::from(NO_FILE)
}

impl ContextVFS<'_> {
    fn check_version(
        &mut self,
        lower_bound: genvm_common::version::Version,
    ) -> Result<(), generated::types::Error> {
        if self.context.data.version >= lower_bound {
            Ok(())
        } else {
            log_warn!(lower_bound = lower_bound, vm_version = self.context.data.version; "version check failed");
            Err(generated::types::Errno::Inval.into())
        }
    }
}

#[allow(unused_variables)]
#[async_trait::async_trait]
impl generated::genlayer_sdk::GenlayerSdk for ContextVFS<'_> {
    async fn gl_call(
        &mut self,
        mem: &mut wiggle::GuestMemory<'_>,
        request: wiggle::GuestPtr<u8>,
        request_len: u32,
    ) -> Result<generated::types::Fd, generated::types::Error> {
        let request = request.as_array(request_len);
        let request = read_owned_vec(mem, request)?;

        let request = match calldata::decode(&request) {
            Err(e) => {
                log_info!(error:ah = &e; "calldata parse failed");

                return Err(generated::types::Errno::Inval.into());
            }
            Ok(v) => v,
        };

        log_trace!(request:serde = request; "gl_call");

        let request: gl_call::Message = match calldata::from_value(request) {
            Ok(v) => v,
            Err(e) => {
                log_info!(error:err = e; "calldata deserialization failed");

                return Err(generated::types::Errno::Inval.into());
            }
        };

        match request {
            gl_call::Message::EthSend {
                address,
                calldata,
                value,
            } => {
                if !self.context.data.conf.is_deterministic {
                    return Err(generated::types::Errno::Forbidden.into());
                }
                if !self.context.data.conf.can_send_messages {
                    return Err(generated::types::Errno::Forbidden.into());
                }

                if !value.is_zero() {
                    let my_balance = self
                        .context
                        .get_balance_impl(self.context.data.message_data.contract_address)
                        .await?;

                    if value + self.context.messages_decremented > my_balance {
                        return Err(generated::types::Errno::Inbalance.into());
                    }
                }

                let data_json = serde_json::json!({
                    "value": format!("0x{:x}", value),
                });
                let data_str = serde_json::to_string(&data_json).unwrap();

                let supervisor = self.context.data.supervisor.clone();
                let res = supervisor
                    .host
                    .lock()
                    .await
                    .eth_send(address, &calldata, &data_str)
                    .map_err(generated::types::Error::trap)?;

                self.context.messages_decremented += value;
                Ok(file_fd_none())
            }
            gl_call::Message::EthCall { address, calldata } => {
                if !self.context.data.conf.is_deterministic {
                    return Err(generated::types::Errno::Forbidden.into());
                }
                if !self.context.data.conf.can_call_others {
                    return Err(generated::types::Errno::Forbidden.into());
                }

                let supervisor = self.context.data.supervisor.clone();
                let res = supervisor
                    .host
                    .lock()
                    .await
                    .eth_call(address, &calldata)
                    .map_err(generated::types::Error::trap)?;
                Ok(generated::types::Fd::from(
                    self.vfs
                        .place_content(vfs::FileContents {
                            contents: util::SharedBytes::new(res),
                            pos: 0,
                            release_memory: true,
                        })
                        .map_err(generated::types::Error::trap)?,
                ))
            }
            gl_call::Message::CallContract {
                address,
                calldata,
                mut state,
            } => {
                if !self.context.data.conf.is_deterministic {
                    return Err(generated::types::Errno::Forbidden.into());
                }
                if !self.context.data.conf.can_call_others {
                    return Err(generated::types::Errno::Forbidden.into());
                }

                if state == public_abi::StorageType::Default {
                    state = public_abi::StorageType::LatestNonFinal;
                }

                let supervisor = self.context.data.supervisor.clone();

                let my_conf = self.context.data.conf;

                let calldata_encoded = calldata::encode(&calldata);

                let mut my_data = self
                    .context
                    .data
                    .message_data
                    .fork(public_abi::EntryKind::Main, calldata_encoded);
                my_data.stack.push(my_data.contract_address);

                let calldata_encoded = calldata::encode(&calldata);

                let vm_data = SingleVMData {
                    conf: base::Config {
                        needs_error_fingerprint: true,
                        is_deterministic: true,
                        can_read_storage: my_conf.can_read_storage,
                        can_write_storage: false,
                        can_spawn_nondet: my_conf.can_spawn_nondet,
                        can_call_others: my_conf.can_call_others,
                        can_send_messages: my_conf.can_send_messages,
                        state_mode: state,
                    },
                    message_data: ExtendedMessage {
                        contract_address: address,
                        sender_address: my_data.sender_address,
                        origin_address: my_data.origin_address,
                        value: num_bigint::BigInt::ZERO,
                        is_init: false,
                        datetime: my_data.datetime,
                        chain_id: my_data.chain_id,
                        entry_kind: my_data.entry_kind,
                        entry_data: my_data.entry_data,
                        entry_stage_data: default_entry_stage_data(),
                        stack: my_data.stack,
                    },
                    storage: rt::vm::storage::Storage::new(
                        address,
                        supervisor.get_storage_limiter(),
                        StorageHostHolder(
                            supervisor.host.clone(),
                            ReadToken {
                                account: address,
                                mode: state,
                            },
                        ),
                    ),
                    supervisor: supervisor.clone(),
                    version: genvm_common::version::Version::ZERO,
                    should_capture_fp: Arc::new(std::sync::atomic::AtomicBool::new(true)),
                };

                let res = self
                    .context
                    .spawn_and_run(&supervisor, vm_data)
                    .await
                    .map_err(generated::types::Error::trap)?;

                self.set_vm_run_result(res).map(|x| x.0)
            }
            gl_call::Message::EmitEvent { topics, blob } => {
                self.check_version(genvm_common::version::Version::new(0, 1, 5))?;

                if !self.context.data.conf.is_deterministic {
                    log_warn!("forbidden emit event in deterministic mode");

                    return Err(generated::types::Errno::Forbidden.into());
                }

                if topics.len() > public_abi::EVENT_MAX_TOPICS as usize {
                    log_warn!(cnt = topics.len(), max = public_abi::EVENT_MAX_TOPICS; "too many topics");
                    return Err(generated::types::Errno::Inval.into());
                }

                let mut real_topics = [[0; 32]; public_abi::EVENT_MAX_TOPICS as usize];

                for (i, gl_call::Bytes(t)) in topics.iter().enumerate() {
                    if t.len() != 32 {
                        log_warn!(len = t.len(); "invalid topic length");

                        return Err(generated::types::Errno::Inval.into());
                    }

                    real_topics[i].copy_from_slice(t);
                }

                let blob_data = calldata::encode(&calldata::Value::Map(blob));

                let supervisor = self.context.data.supervisor.clone();

                let size = topics.len() + (blob_data.len() + 31) / 32;
                let size = size as u64;
                supervisor
                    .get_storage_limiter()
                    .consume(size)
                    .map_err(generated::types::Error::trap)?;

                supervisor
                    .host
                    .lock()
                    .await
                    .post_event(&real_topics[..topics.len()], &blob_data)
                    .map_err(generated::types::Error::trap)?;

                return Ok(file_fd_none());
            }
            gl_call::Message::PostMessage {
                address,
                calldata,
                value,
                on,
            } => {
                if !self.context.data.conf.is_deterministic {
                    return Err(generated::types::Errno::Forbidden.into());
                }
                if !self.context.data.conf.can_send_messages {
                    return Err(generated::types::Errno::Forbidden.into());
                }

                if !value.is_zero() {
                    let my_balance = self
                        .context
                        .get_balance_impl(self.context.data.message_data.contract_address)
                        .await?;

                    if value + self.context.messages_decremented > my_balance {
                        return Err(generated::types::Errno::Inbalance.into());
                    }
                }

                let calldata_encoded = calldata::encode(&calldata);

                let data_json = serde_json::json!({
                    "value": format!("0x{:x}", value),
                    "on": on,
                });
                let data_str = serde_json::to_string(&data_json).unwrap();

                let res = self
                    .context
                    .data
                    .supervisor
                    .host
                    .lock()
                    .await
                    .post_message(&address, &calldata_encoded, &data_str)
                    .map_err(generated::types::Error::trap)?;

                self.context.messages_decremented += value;

                Ok(file_fd_none())
            }
            gl_call::Message::DeployContract {
                calldata,
                code,
                value,
                on,
                salt_nonce,
            } => {
                if !self.context.data.conf.is_deterministic {
                    return Err(generated::types::Errno::Forbidden.into());
                }
                if !self.context.data.conf.can_send_messages {
                    return Err(generated::types::Errno::Forbidden.into());
                }

                if !value.is_zero() {
                    let my_balance = self
                        .context
                        .get_balance_impl(self.context.data.message_data.contract_address)
                        .await?;

                    if value + self.context.messages_decremented > my_balance {
                        return Err(generated::types::Errno::Inbalance.into());
                    }
                }

                let calldata_encoded = calldata::encode(&calldata);

                let data_json = serde_json::json!({
                    "value": format!("0x{:x}", value),
                    "salt_nonce": format!("0x{:x}", salt_nonce),
                    "on": on,
                });
                let data_str = serde_json::to_string(&data_json).unwrap();

                let res = self
                    .context
                    .data
                    .supervisor
                    .host
                    .lock()
                    .await
                    .deploy_contract(&calldata_encoded, &code, &data_str)
                    .map_err(generated::types::Error::trap)?;

                self.context.messages_decremented += value;

                Ok(file_fd_none())
            }
            gl_call::Message::WebRender(render_payload) => {
                if self.context.data.conf.is_deterministic {
                    return Err(generated::types::Errno::Forbidden.into());
                }

                let web = self.context.data.supervisor.modules.web.clone();
                let task = taskify(async move {
                    web.send::<genvm_modules_interfaces::web::RenderAnswer, _>(
                        genvm_modules_interfaces::web::Message::Render(render_payload),
                    )
                    .await
                })
                .await
                .map_err(generated::types::Error::trap)?;

                Ok(generated::types::Fd::from(
                    self.vfs
                        .place_content(vfs::FileContents {
                            contents: util::SharedBytes::new(task),
                            pos: 0,
                            release_memory: true,
                        })
                        .map_err(generated::types::Error::trap)?,
                ))
            }
            gl_call::Message::WebRequest(request_payload) => {
                if self.context.data.conf.is_deterministic {
                    return Err(generated::types::Errno::Forbidden.into());
                }

                let web = self.context.data.supervisor.modules.web.clone();
                let task = taskify(async move {
                    web.send::<genvm_modules_interfaces::web::RenderAnswer, _>(
                        genvm_modules_interfaces::web::Message::Request(request_payload),
                    )
                    .await
                })
                .await
                .map_err(generated::types::Error::trap)?;

                Ok(generated::types::Fd::from(
                    self.vfs
                        .place_content(vfs::FileContents {
                            contents: util::SharedBytes::new(task),
                            pos: 0,
                            release_memory: true,
                        })
                        .map_err(generated::types::Error::trap)?,
                ))
            }
            gl_call::Message::ExecPrompt(prompt_payload) => {
                if self.context.data.conf.is_deterministic {
                    return Err(generated::types::Errno::Forbidden.into());
                }

                if prompt_payload.images.len() > 2 {
                    return Err(generated::types::Errno::Inval.into());
                }

                let remaining_fuel_as_gen = self
                    .context
                    .data
                    .supervisor
                    .host
                    .lock()
                    .await
                    .remaining_fuel_as_gen()
                    .map_err(generated::types::Error::trap)?;

                let sup = self.context.data.supervisor.clone();

                let task = taskify(async move {
                    let result = sup
                        .modules
                        .llm
                        .send::<genvm_modules_interfaces::llm::PromptAnswer, _>(
                            genvm_modules_interfaces::llm::Message::Prompt {
                                payload: prompt_payload,
                                remaining_fuel_as_gen,
                            },
                        )
                        .await?;

                    use genvm_modules_interfaces::llm::PromptAnswer;

                    if let Ok(PromptAnswer { consumed_gen, .. }) = &result {
                        sup.host
                            .lock()
                            .await
                            .consume_fuel(*consumed_gen)
                            .map_err(generated::types::Error::trap)?;
                    }

                    Ok(result.map(|r| r.data))
                })
                .await
                .map_err(generated::types::Error::trap)?;

                Ok(generated::types::Fd::from(
                    self.vfs
                        .place_content(vfs::FileContents {
                            contents: util::SharedBytes::new(task),
                            pos: 0,
                            release_memory: true,
                        })
                        .map_err(generated::types::Error::trap)?,
                ))
            }
            gl_call::Message::ExecPromptTemplate(prompt_template_payload) => {
                if self.context.data.conf.is_deterministic {
                    return Err(generated::types::Errno::Forbidden.into());
                }

                let expect_bool = !matches!(
                    &prompt_template_payload,
                    genvm_modules_interfaces::llm::PromptTemplatePayload::EqNonComparativeLeader(_)
                );

                // Get remaining fuel from host
                let remaining_fuel_as_gen = self
                    .context
                    .data
                    .supervisor
                    .host
                    .lock()
                    .await
                    .remaining_fuel_as_gen()
                    .map_err(generated::types::Error::trap)?;

                let sup = self.context.data.supervisor.clone();
                let task = taskify(async move {
                    let answer = sup
                        .modules
                        .llm
                        .send::<genvm_modules_interfaces::llm::PromptAnswer, _>(
                            genvm_modules_interfaces::llm::Message::PromptTemplate {
                                payload: prompt_template_payload,
                                remaining_fuel_as_gen,
                            },
                        )
                        .await?;
                    use genvm_modules_interfaces::llm::{PromptAnswer, PromptAnswerData};

                    if let Ok(PromptAnswer { consumed_gen, .. }) = &answer {
                        sup.host
                            .lock()
                            .await
                            .consume_fuel(*consumed_gen)
                            .map_err(generated::types::Error::trap)?;
                    }

                    match (expect_bool, answer) {
                        (_, Err(e)) => Ok(Err(e)),
                        (
                            true,
                            Ok(PromptAnswer {
                                data: PromptAnswerData::Bool(answer),
                                consumed_gen,
                            }),
                        ) => Ok(Ok(PromptAnswerData::Bool(answer))),
                        (
                            false,
                            Ok(PromptAnswer {
                                data: PromptAnswerData::Text(answer),
                                consumed_gen,
                            }),
                        ) => Ok(Ok(PromptAnswerData::Text(answer))),
                        (_, Ok(_)) => Err(anyhow::anyhow!("unmatched result")),
                    }
                })
                .await
                .map_err(generated::types::Error::trap)?;

                Ok(generated::types::Fd::from(
                    self.vfs
                        .place_content(vfs::FileContents {
                            contents: util::SharedBytes::new(task),
                            pos: 0,
                            release_memory: true,
                        })
                        .map_err(generated::types::Error::trap)?,
                ))
            }
            gl_call::Message::Rollback(msg) => Err(generated::types::Error::trap(
                rt::errors::UserError(msg).into(),
            )),
            gl_call::Message::Return(value) => {
                let ret = calldata::encode(&value);

                // for return we are not interested in it
                self.context
                    .data
                    .should_capture_fp
                    .store(false, std::sync::atomic::Ordering::Relaxed);

                Err(generated::types::Error::trap(ContractReturn(ret).into()))
            }
            gl_call::Message::RunNondet {
                data_leader,
                data_validator,
            } => self.run_nondet(data_leader, data_validator).await,
            gl_call::Message::Sandbox {
                data,
                allow_write_ops,
            } => self.sandbox(data, allow_write_ops).await,
            gl_call::Message::Trace(message) => self.gl_call_trace(message).await,
        }
    }

    async fn storage_read(
        &mut self,
        mem: &mut wiggle::GuestMemory<'_>,
        slot: wiggle::GuestPtr<u8>,
        index: u32,
        buf: wiggle::GuestPtr<u8>,
        buf_len: u32,
    ) -> Result<(), generated::types::Error> {
        let buf = buf.as_array(buf_len);

        if !self.context.data.conf.is_deterministic {
            return Err(generated::types::Errno::Forbidden.into());
        }
        if !self.context.data.conf.can_read_storage {
            return Err(generated::types::Errno::Forbidden.into());
        }

        if index.checked_add(buf_len).is_none() {
            return Err(generated::types::Errno::Inval.into());
        }

        let account = self.context.data.message_data.contract_address;

        let slot = SlotID::read_from_mem(mem, slot)?;
        let mem_size = buf_len as usize;
        let mut vec = Vec::with_capacity(mem_size);
        unsafe { vec.set_len(mem_size) };

        if self.context.data.conf.state_mode == public_abi::StorageType::Default {
            self.context
                .data
                .storage
                .read(slot, index, &mut vec)
                .await
                .map_err(generated::types::Error::trap)?;
        } else {
            self.context
                .data
                .supervisor
                .host
                .lock()
                .await
                .storage_read(
                    self.context.data.conf.state_mode,
                    account,
                    slot,
                    index,
                    &mut vec,
                )
                .map_err(generated::types::Error::trap)?;
        }

        mem.copy_from_slice(&vec, buf)?;
        Ok(())
    }

    async fn storage_write(
        &mut self,
        mem: &mut wiggle::GuestMemory<'_>,
        slot: wiggle::GuestPtr<u8>,
        index: u32,
        buf: wiggle::GuestPtr<u8>,
        buf_len: u32,
    ) -> Result<(), generated::types::Error> {
        let buf = buf.as_array(buf_len);

        if !self.context.data.conf.is_deterministic {
            return Err(generated::types::Errno::Forbidden.into());
        }
        if !self.context.data.conf.can_write_storage {
            return Err(generated::types::Errno::Forbidden.into());
        }

        if index.checked_add(buf_len).is_none() {
            return Err(generated::types::Errno::Inval.into());
        }

        let slot = SlotID::read_from_mem(mem, slot)?;

        if self.context.data.supervisor.locked_slots.contains(slot) {
            return Err(generated::types::Errno::Forbidden.into());
        }

        let ptr = mem.as_cow(buf)?;

        self.context
            .data
            .storage
            .write(slot, index, &ptr)
            .await
            .map_err(generated::types::Error::trap)
    }

    async fn get_balance(
        &mut self,
        mem: &mut wiggle::GuestMemory<'_>,
        account: wiggle::GuestPtr<u8>,
        result: wiggle::GuestPtr<u8>,
    ) -> Result<(), generated::types::Error> {
        let address = read_addr_from_mem(mem, account)?;

        self.context
            .get_balance_impl_wasi(mem, address, result, false)
            .await
    }

    async fn get_self_balance(
        &mut self,
        mem: &mut wiggle::GuestMemory<'_>,
        result: wiggle::GuestPtr<u8>,
    ) -> Result<(), generated::types::Error> {
        if !self.context.data.conf.is_deterministic {
            return Err(generated::types::Errno::Forbidden.into());
        }

        self.context
            .get_balance_impl_wasi(
                mem,
                self.context.data.message_data.contract_address,
                result,
                true,
            )
            .await
    }
}

impl Context {
    async fn get_balance_impl_wasi(
        &mut self,
        mem: &mut wiggle::GuestMemory<'_>,
        address: calldata::Address,
        result: wiggle::GuestPtr<u8>,
        is_self: bool,
    ) -> Result<(), generated::types::Error> {
        let mut res = self.get_balance_impl(address).await?;

        if is_self && self.data.conf.is_main() {
            res -= self.messages_decremented;
        }

        let res = res.to_little_endian();
        mem.copy_from_slice(&res, result.as_array(32))?;

        Ok(())
    }

    pub async fn get_balance_impl(
        &mut self,
        address: calldata::Address,
    ) -> Result<primitive_types::U256, generated::types::Error> {
        if let Some(res) = self.data.supervisor.balances.get(&address) {
            return Ok(*res);
        }

        let res = self
            .data
            .supervisor
            .host
            .lock()
            .await
            .get_balance(address)
            .map_err(generated::types::Error::trap)?;

        let _ = self.data.supervisor.balances.insert(address, res);

        Ok(res)
    }

    pub fn log(&self) -> calldata::Value {
        let msg = calldata::to_value(&self.data.message_data).unwrap();
        let conf = calldata::to_value(&self.data.conf).unwrap();

        calldata::Value::Map(BTreeMap::from([
            ("config".to_owned(), conf),
            ("message".to_owned(), msg),
        ]))
    }

    async fn spawn_and_run(
        &mut self,
        supervisor: &Arc<rt::supervisor::Supervisor>,
        essential_data: SingleVMData,
    ) -> anyhow::Result<rt::vm::RunOk> {
        let limiter = self
            .data
            .supervisor
            .limiter
            .get(essential_data.conf.is_deterministic)
            .derived();

        let vm = rt::supervisor::spawn(supervisor, essential_data, limiter).await;
        let vm = match vm {
            Ok(vm) => rt::supervisor::apply_contract_actions(supervisor, vm).await,
            Err(e) => Err(e),
        };
        let result = match vm {
            Ok(vm) => vm.run().await,
            Err(e) => Err(e),
        };

        result.map(|x| x.run_ok)
    }
}

impl ContextVFS<'_> {
    async fn gl_call_trace(
        &mut self,
        msg: gl_call::TracePayload,
    ) -> Result<generated::types::Fd, generated::types::Error> {
        self.check_version(genvm_common::version::Version::new(0, 1, 10))?;
        match msg {
            gl_call::TracePayload::Message(text) => {
                let now = std::time::Instant::now();
                let since_prev = now.duration_since(self.context.prev_time);
                self.context.prev_time = now;

                log_info!(
                    message = text,
                    elapsed:? = now.duration_since(self.context.start_time),
                    since_last_trace:? = since_prev;
                    "trace"
                );

                Ok(file_fd_none())
            }
            gl_call::TracePayload::RuntimeMicroSec => {
                let elapsed_micros = if self.context.data.conf.is_deterministic
                    && !self.context.data.supervisor.shared_data.debug_mode
                {
                    0u64
                } else {
                    let elapsed = std::time::Instant::now().duration_since(self.context.start_time);
                    elapsed.as_micros() as u64
                };

                let data = calldata::encode(&calldata::Value::Number(num_bigint::BigInt::from(
                    elapsed_micros,
                )));
                Ok(generated::types::Fd::from(
                    self.vfs
                        .place_content(vfs::FileContents {
                            contents: util::SharedBytes::new(data),
                            pos: 0,
                            release_memory: true,
                        })
                        .map_err(generated::types::Error::trap)?,
                ))
            }
        }
    }

    async fn run_nondet(
        &mut self,
        data_leader: Vec<u8>,
        data_validator: Vec<u8>,
    ) -> Result<generated::types::Fd, generated::types::Error> {
        if !self.context.data.conf.can_spawn_nondet {
            return Err(generated::types::Errno::Forbidden.into());
        }

        let call_no = self
            .context
            .data
            .supervisor
            .nondet_call_no
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        let leaders_res = self
            .context
            .data
            .supervisor
            .host
            .lock()
            .await
            .get_leader_result(call_no)
            .map_err(generated::types::Error::trap)?;

        let result_to_return = if self.context.data.supervisor.shared_data.is_sync {
            match leaders_res {
                None => {
                    return Err(generated::types::Error::trap(anyhow::anyhow!(
                        "absent leader result in sync mode, call_no: {}",
                        call_no
                    )))
                }
                Some(v) => v,
            }
        } else {
            let storage_checkpoint = self.context.data.storage.clone();

            let message_data = match &leaders_res {
                None => self.context.data.message_data.fork_leader(
                    public_abi::EntryKind::ConsensusStage,
                    data_leader,
                    None,
                ),
                Some(leaders_res) => {
                    let dup = match leaders_res {
                        rt::vm::RunOk::Return(items) => rt::vm::RunOk::Return(items.clone()),
                        rt::vm::RunOk::UserError(msg) => rt::vm::RunOk::UserError(msg.clone()),
                        rt::vm::RunOk::VMError(msg, _) => rt::vm::RunOk::VMError(msg.clone(), None),
                    };
                    self.context.data.message_data.fork_leader(
                        public_abi::EntryKind::ConsensusStage,
                        data_validator,
                        Some(dup),
                    )
                }
            };

            let supervisor = self.context.data.supervisor.clone();

            let vm_data = SingleVMData {
                conf: base::Config {
                    needs_error_fingerprint: false,
                    is_deterministic: false,
                    can_read_storage: self.context.data.conf.can_read_storage,
                    can_write_storage: false,
                    can_spawn_nondet: false,
                    can_call_others: false,
                    can_send_messages: false,
                    state_mode: public_abi::StorageType::Default,
                },
                message_data,
                version: self.context.data.version,
                supervisor: supervisor.clone(),
                should_capture_fp: Arc::new(std::sync::atomic::AtomicBool::new(false)),
                storage: storage_checkpoint,
            };

            let task_done = Arc::new(tokio::sync::Notify::new());
            let task = rt::supervisor::NonDetVMTask {
                task: vm_data,
                call_no,
                tasks_done: task_done.clone(),
            };

            match leaders_res {
                None => {
                    let res = task
                        .run_now(&self.context.data.supervisor)
                        .await
                        .map_err(generated::types::Error::trap)?;

                    self.context
                        .data
                        .supervisor
                        .host
                        .lock()
                        .await
                        .post_nondet_result(call_no, &res)
                        .map_err(generated::types::Error::trap)?;

                    res
                }
                Some(leaders_res) => {
                    rt::supervisor::submit_nondet_vm_task(&self.context.data.supervisor, task)
                        .await;

                    leaders_res
                }
            }
        };

        self.set_vm_run_result(result_to_return).map(|x| x.0)
    }

    async fn sandbox(
        &mut self,
        data: Vec<u8>,
        allow_write_ops: bool,
    ) -> Result<generated::types::Fd, generated::types::Error> {
        let supervisor = self.context.data.supervisor.clone();

        let message_data = self
            .context
            .data
            .message_data
            .fork(public_abi::EntryKind::Sandbox, data);

        let zelf_conf = &self.context.data.conf;

        let storage_checkpoint = self.context.data.storage.clone();

        let vm_data = SingleVMData {
            conf: base::Config {
                needs_error_fingerprint: false,
                is_deterministic: zelf_conf.is_deterministic,
                can_read_storage: zelf_conf.can_read_storage,
                can_write_storage: zelf_conf.can_write_storage & allow_write_ops,
                can_spawn_nondet: false,
                can_call_others: false,
                can_send_messages: zelf_conf.can_send_messages & allow_write_ops,
                state_mode: zelf_conf.state_mode,
            },
            message_data,
            supervisor: supervisor.clone(),
            version: genvm_common::version::Version::ZERO,
            should_capture_fp: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            storage: storage_checkpoint,
        };

        let my_res = self.context.spawn_and_run(&supervisor, vm_data).await;
        let my_res = match my_res {
            Ok(res) => Ok(res),
            Err(e) => rt::errors::unwrap_vm_errors(e),
        }
        .map_err(generated::types::Error::trap)?;

        let data: Box<[u8]> = my_res.as_bytes_iter().collect();
        Ok(generated::types::Fd::from(
            self.vfs
                .place_content(vfs::FileContents {
                    contents: util::SharedBytes::new(data),
                    pos: 0,
                    release_memory: true,
                })
                .map_err(generated::types::Error::trap)?,
        ))
    }
}
