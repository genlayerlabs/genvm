use std::collections::BTreeMap;
use std::sync::Arc;

use genvm_common::*;

use genvm_modules_interfaces::GenericValue;
use wiggle::GuestError;

use crate::host::SlotID;
use crate::public_abi;
use crate::{
    calldata,
    errors::*,
    ustar::SharedBytes,
    vm::{self, RunOk},
};

use super::{base, common::*, gl_call};

fn entry_kind_as_int<S>(data: &public_abi::EntryKind, d: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    d.serialize_u8(*data as u8)
}

#[derive(serde::Serialize, Debug)]
pub struct TransformedMessage {
    pub contract_address: calldata::Address,
    pub sender_address: calldata::Address,
    pub origin_address: calldata::Address,
    pub stack: Vec<calldata::Address>,

    pub chain_id: num_bigint::BigInt,
    pub value: num_bigint::BigInt,
    pub is_init: bool,
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

impl TransformedMessage {
    pub fn fork_leader(
        &self,
        entry_kind: public_abi::EntryKind,
        entry_data: Vec<u8>,
        entry_leader_data: Option<RunOk>,
    ) -> Self {
        let entry_leader_data = match entry_leader_data {
            None => default_entry_stage_data(),
            Some(entry_leader_data) => calldata::Value::Map(BTreeMap::from([(
                "leaders_result".into(),
                calldata::Value::Bytes(Vec::from_iter(entry_leader_data.as_bytes_iter())),
            )])),
        };

        TransformedMessage {
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

pub struct SingleVMData {
    pub conf: base::Config,
    pub message_data: TransformedMessage,
    pub supervisor: Arc<tokio::sync::Mutex<crate::vm::Supervisor>>,
    pub version: genvm_common::version::Version,
}

pub struct Context {
    pub data: SingleVMData,
    pub shared_data: Arc<vm::SharedData>,
    pub messages_decremented: primitive_types::U256,
}

pub struct ContextVFS<'a> {
    pub(super) vfs: &'a mut VFS,
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
    pub fn new(data: SingleVMData, shared_data: Arc<vm::SharedData>) -> Self {
        Self {
            data,
            shared_data,
            messages_decremented: primitive_types::U256::zero(),
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
            PtrOverflow { .. } | PtrOutOfBounds { .. } | PtrNotAligned { .. } => {
                generated::types::Error::trap(err.into())
            }
            PtrBorrowed { .. } => generated::types::Errno::Fault.into(),
            InvalidUtf8 { .. } => generated::types::Errno::Ilseq.into(),
            TryFromIntError { .. } => generated::types::Errno::Overflow.into(),
            SliceLengthsDiffer { .. } => generated::types::Errno::Fault.into(),
            BorrowCheckerOutOfHandles { .. } => generated::types::Errno::Fault.into(),
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
        data: vm::RunOk,
    ) -> Result<(generated::types::Fd, usize), generated::types::Error> {
        let data = match data {
            RunOk::VMError(e, cause) => {
                return Err(generated::types::Error::trap(
                    ContractError(e, cause).into(),
                ))
            }
            data => data,
        };
        let data: Box<[u8]> = data.as_bytes_iter().collect();
        let len = data.len();
        Ok((
            generated::types::Fd::from(self.vfs.place_content(
                FileContentsUnevaluated::from_contents(SharedBytes::new(data), 0),
            )),
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
        if self.context.data.version.is_greater_eq_than(lower_bound) {
            Ok(())
        } else {
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
                let mut supervisor = supervisor.lock().await;
                let res = supervisor
                    .host
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
                let mut supervisor = supervisor.lock().await;
                let res = supervisor
                    .host
                    .eth_call(address, &calldata)
                    .map_err(generated::types::Error::trap)?;
                Ok(generated::types::Fd::from(self.vfs.place_content(
                    FileContentsUnevaluated::from_contents(SharedBytes::new(res), 0),
                )))
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
                        is_deterministic: true,
                        can_read_storage: my_conf.can_read_storage,
                        can_write_storage: false,
                        can_spawn_nondet: my_conf.can_spawn_nondet,
                        can_call_others: my_conf.can_call_others,
                        can_send_messages: my_conf.can_send_messages,
                        state_mode: state,
                    },
                    message_data: TransformedMessage {
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
                    supervisor: supervisor.clone(),
                    version: genvm_common::version::Version::ZERO,
                };

                let res = self
                    .context
                    .spawn_and_run(&supervisor, vm_data)
                    .await
                    .map_err(generated::types::Error::trap)?;

                self.set_vm_run_result(res).map(|x| x.0)
            }
            gl_call::Message::EmitEvent { topics, blob } => {
                self.check_version(genvm_common::version::Version::new(0, 2, 0))?;

                if !self.context.data.conf.is_deterministic {
                    return Err(generated::types::Errno::Forbidden.into());
                }

                if topics.len() > public_abi::EVENT_MAX_TOPICS as usize {
                    return Err(generated::types::Errno::Inval.into());
                }

                for c in &topics {
                    if c.0.len() != 32 {
                        return Err(generated::types::Errno::Inval.into());
                    }
                }

                let supervisor = self.context.data.supervisor.clone();
                let supervisor = supervisor.lock().await;

                // todo
                _ = supervisor;

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

                let supervisor = self.context.data.supervisor.clone();
                let mut supervisor = supervisor.lock().await;
                let res = supervisor
                    .host
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

                let supervisor = self.context.data.supervisor.clone();
                let mut supervisor = supervisor.lock().await;
                let res = supervisor
                    .host
                    .deploy_contract(&calldata_encoded, &code, &data_str)
                    .map_err(generated::types::Error::trap)?;

                self.context.messages_decremented += value;

                Ok(file_fd_none())
            }
            gl_call::Message::WebRender(render_payload) => {
                if self.context.data.conf.is_deterministic {
                    return Err(generated::types::Errno::Forbidden.into());
                }

                let web = self.context.shared_data.modules.web.clone();
                let task = tokio::spawn(taskify(async move {
                    web.send::<genvm_modules_interfaces::web::RenderAnswer, _>(
                        genvm_modules_interfaces::web::Message::Render(render_payload),
                    )
                    .await
                }));

                Ok(generated::types::Fd::from(
                    self.vfs
                        .place_content(FileContentsUnevaluated::from_task(task)),
                ))
            }
            gl_call::Message::WebRequest(request_payload) => {
                if self.context.data.conf.is_deterministic {
                    return Err(generated::types::Errno::Forbidden.into());
                }

                let web = self.context.shared_data.modules.web.clone();
                let task = tokio::spawn(taskify(async move {
                    web.send::<genvm_modules_interfaces::web::RenderAnswer, _>(
                        genvm_modules_interfaces::web::Message::Request(request_payload),
                    )
                    .await
                }));

                Ok(generated::types::Fd::from(
                    self.vfs
                        .place_content(FileContentsUnevaluated::from_task(task)),
                ))
            }
            gl_call::Message::ExecPrompt(prompt_payload) => {
                if self.context.data.conf.is_deterministic {
                    return Err(generated::types::Errno::Forbidden.into());
                }

                let llm = self.context.shared_data.modules.llm.clone();
                let task = tokio::spawn(taskify(async move {
                    llm.send::<genvm_modules_interfaces::llm::PromptAnswer, _>(
                        genvm_modules_interfaces::llm::Message::Prompt(prompt_payload),
                    )
                    .await
                }));

                Ok(generated::types::Fd::from(
                    self.vfs
                        .place_content(FileContentsUnevaluated::from_task(task)),
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

                let llm = self.context.shared_data.modules.llm.clone();
                let task = tokio::spawn(taskify(async move {
                    let answer = llm
                        .send::<genvm_modules_interfaces::llm::PromptAnswer, _>(
                            genvm_modules_interfaces::llm::Message::PromptTemplate(
                                prompt_template_payload,
                            ),
                        )
                        .await?;
                    use genvm_modules_interfaces::llm::PromptAnswer;
                    match (expect_bool, answer) {
                        (_, Err(e)) => Ok(Err(e)),
                        (true, Ok(PromptAnswer::Bool(answer))) => {
                            Ok(Ok(PromptAnswer::Bool(answer)))
                        }
                        (false, Ok(PromptAnswer::Text(answer))) => {
                            Ok(Ok(PromptAnswer::Text(answer)))
                        }
                        (_, Ok(_)) => Err(anyhow::anyhow!("unmatched result")),
                    }
                }));

                Ok(generated::types::Fd::from(
                    self.vfs
                        .place_content(FileContentsUnevaluated::from_task(task)),
                ))
            }
            gl_call::Message::Rollback(msg) => {
                Err(generated::types::Error::trap(UserError(msg).into()))
            }
            gl_call::Message::Return(value) => {
                let ret = calldata::encode(&value);
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

        let supervisor = self.context.data.supervisor.clone();
        let mut supervisor = supervisor.lock().await;

        supervisor
            .host
            .storage_read(
                self.context.data.conf.state_mode,
                account,
                slot,
                index,
                &mut vec,
            )
            .map_err(generated::types::Error::trap)?;

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

        let account = self.context.data.message_data.contract_address;
        let slot = SlotID::read_from_mem(mem, slot)?;

        if self.context.shared_data.locked_slots.contains(slot) {
            return Err(generated::types::Errno::Forbidden.into());
        }

        let buf: Vec<u8> = read_owned_vec(mem, buf)?;

        let supervisor = self.context.data.supervisor.clone();
        let mut supervisor = supervisor.lock().await;

        supervisor
            .host
            .storage_write(account, slot, index, &buf)
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
        if let Some(res) = self.shared_data.balances.get(&address) {
            return Ok(*res);
        }

        let supervisor = self.data.supervisor.clone();
        let mut supervisor = supervisor.lock().await;
        let res = supervisor
            .host
            .get_balance(address)
            .map_err(generated::types::Error::trap)?;

        let _ = self.shared_data.balances.insert(address, res);

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
        supervisor: &Arc<tokio::sync::Mutex<crate::vm::Supervisor>>,
        essential_data: SingleVMData,
    ) -> vm::RunResult {
        let limiter = if essential_data.conf.is_deterministic {
            self.shared_data.limiter_det.clone()
        } else {
            self.shared_data.limiter_non_det.clone()
        };

        let (mut vm, instance, limiter_save) = {
            let mut supervisor = supervisor.lock().await;

            let mut vm = supervisor.spawn(essential_data).await?;
            let instance = supervisor.apply_contract_actions(&mut vm).await?;

            (vm, instance, limiter.save())
        };
        let result = vm.run(&instance).await;

        limiter.restore(limiter_save);

        result
    }
}

impl ContextVFS<'_> {
    async fn run_nondet(
        &mut self,
        data_leader: Vec<u8>,
        data_validator: Vec<u8>,
    ) -> Result<generated::types::Fd, generated::types::Error> {
        if !self.context.data.conf.can_spawn_nondet {
            return Err(generated::types::Errno::Forbidden.into());
        }

        // relaxed reason: only deterministic VM can run it
        let call_no = self
            .context
            .shared_data
            .nondet_call_no
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let leaders_res = {
            let supervisor = self.context.data.supervisor.clone();
            let mut supervisor = supervisor.lock().await;
            supervisor.host.get_leader_result(call_no)
        }
        .map_err(generated::types::Error::trap)?;

        let leaders_res = match (leaders_res, self.context.shared_data.is_sync) {
            (leaders_res, false) => leaders_res,
            (Some(leaders_res), true) => return self.set_vm_run_result(leaders_res).map(|x| x.0),
            (_, true) => {
                return Err(generated::types::Error::trap(anyhow::anyhow!(
                    "absent leader result in sync mode"
                )))
            }
        };

        let message_data = match &leaders_res {
            None => self.context.data.message_data.fork_leader(
                public_abi::EntryKind::ConsensusStage,
                data_leader,
                None,
            ),
            Some(leaders_res) => {
                let dup = match leaders_res {
                    RunOk::Return(items) => RunOk::Return(items.clone()),
                    RunOk::UserError(msg) => RunOk::UserError(msg.clone()),
                    RunOk::VMError(msg, _) => RunOk::VMError(msg.clone(), None),
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
                is_deterministic: false,
                can_read_storage: false,
                can_write_storage: false,
                can_spawn_nondet: false,
                can_call_others: false,
                can_send_messages: false,
                state_mode: public_abi::StorageType::Default,
            },
            message_data,
            version: self.context.data.version,
            supervisor: supervisor.clone(),
        };

        let my_res = self.context.spawn_and_run(&supervisor, vm_data).await;
        let my_res = ContractError::unwrap_res(my_res).map_err(generated::types::Error::trap)?;

        let ret_res = match leaders_res {
            None => {
                let mut supervisor = supervisor.lock().await;
                supervisor
                    .host
                    .post_nondet_result(call_no, &my_res)
                    .map_err(generated::types::Error::trap)?;
                Ok(my_res)
            }
            Some(leaders_res) => match my_res {
                RunOk::Return(v) if v == [16] => Ok(leaders_res),
                RunOk::Return(v) if v == [8] => {
                    Err(ContractError(format!("validator_disagrees call {}", call_no), None).into())
                }
                _ => {
                    log_warn!(validator_result:? = my_res, leaders_result:? = leaders_res; "validator reported unexpected result");
                    Err(ContractError(format!("validator_disagrees call {}", call_no), None).into())
                }
            },
        };
        let ret_res = ret_res.map_err(generated::types::Error::trap)?;
        self.set_vm_run_result(ret_res).map(|x| x.0)
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

        let vm_data = SingleVMData {
            conf: base::Config {
                is_deterministic: zelf_conf.is_deterministic,
                can_read_storage: false,
                can_write_storage: zelf_conf.can_write_storage & allow_write_ops,
                can_spawn_nondet: false,
                can_call_others: false,
                can_send_messages: zelf_conf.can_send_messages & allow_write_ops,
                state_mode: zelf_conf.state_mode,
            },
            message_data,
            supervisor: supervisor.clone(),
            version: genvm_common::version::Version::ZERO,
        };

        let my_res = self.context.spawn_and_run(&supervisor, vm_data).await;
        let my_res = ContractError::unwrap_res(my_res).map_err(generated::types::Error::trap)?;

        let data: Box<[u8]> = my_res.as_bytes_iter().collect();
        Ok(generated::types::Fd::from(self.vfs.place_content(
            FileContentsUnevaluated::from_contents(SharedBytes::new(data), 0),
        )))
    }
}
