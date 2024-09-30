use core::str;
use std::{
    ffi::{CStr, CString},
    sync::{Arc, Mutex},
};

use serde::Deserialize;
use wiggle::GuestError;

use crate::{
    vm::{self, InitActions},
    Address, Host, MessageData,
};

use super::{base, common::*};

pub struct EssentialGenlayerSdkData {
    pub conf: base::Config,
    pub message_data: MessageData,
    pub entrypoint: Arc<[u8]>,
    pub supervisor: Arc<Mutex<crate::vm::Supervisor>>,
    pub init_actions: InitActions,
}

pub struct Context {
    pub data: EssentialGenlayerSdkData,
    pub shared_data: Arc<vm::SharedData>,
}

pub struct ContextVFS<'a> {
    pub(super) vfs: &'a mut VFS,
    pub(super) context: &'a mut Context,
}

pub(crate) mod generated {
    wiggle::from_witx!({
        witx: ["$CARGO_MANIFEST_DIR/src/wasi/witx/genlayer_sdk.witx"],
        errors: { errno => trappable Error },
        wasmtime: false,
    });

    wiggle::wasmtime_integration!({
        witx: ["$CARGO_MANIFEST_DIR/src/wasi/witx/genlayer_sdk.witx"],
        errors: { errno => trappable Error },
        target: self,
    });
}

impl crate::Address {
    fn read_from_mem(
        addr: &generated::types::Addr,
        mem: &mut wiggle::GuestMemory<'_>,
    ) -> Result<Self, generated::types::Error> {
        let cow = mem.as_cow(addr.ptr.as_array(crate::Address::len().try_into().unwrap()))?;
        let mut ret = Address::new();
        for (x, y) in ret.0.iter_mut().zip(cow.iter()) {
            *x = *y;
        }
        Ok(ret)
    }
}

impl generated::types::Bytes {
    #[allow(dead_code)]
    fn read_owned(
        &self,
        mem: &mut wiggle::GuestMemory<'_>,
    ) -> Result<Vec<u8>, generated::types::Error> {
        Ok(mem.as_cow(self.buf.as_array(self.buf_len))?.into_owned())
    }
}

impl Context {
    pub fn new(data: EssentialGenlayerSdkData, shared_data: Arc<vm::SharedData>) -> Self {
        Self { data, shared_data }
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

pub(super) fn add_to_linker_sync<'a, T: Send + 'static, F>(
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
        fn call<'a>(&self, arg: &'a mut T) -> impl generated::genlayer_sdk::GenlayerSdk {
            self.0.call(arg)
        }
    }
    generated::add_genlayer_sdk_to_linker(linker, Fwd(f))?;
    Ok(())
}

#[derive(Debug)]
pub struct Rollback(pub String);

impl std::error::Error for Rollback {}

impl std::fmt::Display for Rollback {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Rolled back with {}", self.0)
    }
}

#[derive(Debug)]
pub struct ContractReturn(pub Vec<u8>);

impl std::error::Error for ContractReturn {}

impl std::fmt::Display for ContractReturn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Returned {:?}", self.0)
    }
}

impl Context {
    fn ensure_det(&self) -> Result<(), generated::types::Error> {
        if self.data.conf.is_deterministic {
            Ok(())
        } else {
            Err(generated::types::Errno::DeterministicViolation.into())
        }
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
    fn from(err: std::num::TryFromIntError) -> Self {
        match err {
            _ => generated::types::Errno::Overflow.into(),
        }
    }
}

impl From<serde_json::Error> for generated::types::Error {
    fn from(err: serde_json::Error) -> Self {
        match err {
            _ => generated::types::Errno::Io.into(),
        }
    }
}

impl ContextVFS<'_> {
    fn set_vm_run_result(
        &mut self,
        data: vm::RunOk,
    ) -> Result<generated::types::Fd, generated::types::Error> {
        let data: Arc<[u8]> = match data {
            vm::RunOk::Return(buf) => buf.into(),
            vm::RunOk::Rollback(buf) => buf.into_bytes().into(),
        };
        Ok(generated::types::Fd::from(self.vfs.place_content(
            FileContentsUnevaluated::from_contents(data, 0),
        )))
    }
}

#[allow(unused_variables)]
impl generated::genlayer_sdk::GenlayerSdk for ContextVFS<'_> {
    fn get_message_data(
        &mut self,
        mem: &mut wiggle::GuestMemory<'_>,
    ) -> Result<generated::types::ResultNow, generated::types::Error> {
        let res = serde_json::to_vec(&self.context.data.message_data)?;
        let res: Arc<[u8]> = Arc::from(res);
        let len = res.len().try_into()?;
        let fd = self
            .vfs
            .place_content(FileContentsUnevaluated::from_contents(res, 0));
        Ok(generated::types::ResultNow {
            len,
            file: generated::types::Fd::from(fd),
        })
    }

    fn get_entrypoint(
        &mut self,
        mem: &mut wiggle::GuestMemory<'_>,
    ) -> Result<generated::types::ResultNow, generated::types::Error> {
        let res = self.context.data.entrypoint.clone();
        let len = res.len().try_into()?;
        let fd = self
            .vfs
            .place_content(FileContentsUnevaluated::from_contents(res, 0));
        Ok(generated::types::ResultNow {
            len,
            file: generated::types::Fd::from(fd),
        })
    }

    fn rollback(
        &mut self,
        mem: &mut wiggle::GuestMemory<'_>,
        message: wiggle::GuestPtr<str>,
    ) -> anyhow::Error {
        match super::common::read_string(mem, message) {
            Err(e) => e.into(),
            Ok(str) => Rollback(str).into(),
        }
    }

    fn contract_return(
        &mut self,
        mem: &mut wiggle::GuestMemory<'_>,
        message: &generated::types::Bytes,
    ) -> anyhow::Error {
        let res = message.read_owned(mem);
        let Ok(res) = res else {
            return res.unwrap_err().into();
        };
        ContractReturn(res).into()
    }

    fn get_webpage(
        &mut self,
        mem: &mut wiggle::GuestMemory<'_>,
        config: wiggle::GuestPtr<str>,
        url: wiggle::GuestPtr<str>,
    ) -> Result<generated::types::Fd, generated::types::Error> {
        if self.context.data.conf.is_deterministic {
            return Err(generated::types::Errno::DeterministicViolation.into());
        }
        let config_str = read_string(mem, config)?;
        let config_str = CString::new(config_str).map_err(|e| generated::types::Errno::Inval)?;
        let url_str = read_string(mem, url)?;
        let url_str = CString::new(url_str).map_err(|e| generated::types::Errno::Inval)?;

        let supervisor = self.context.data.supervisor.clone();
        let Ok(mut supervisor) = supervisor.lock() else {
            return Err(generated::types::Errno::Io.into());
        };
        let mut fuel = self.context.shared_data.fuel_descriptor.get_fuel();
        let init_fuel = fuel;
        let res = supervisor.modules.web.get_webpage(
            &mut fuel,
            config_str.as_bytes().as_ptr(),
            url_str.as_bytes().as_ptr(),
        );
        self.context
            .shared_data
            .fuel_descriptor
            .consume_fuel(init_fuel - fuel);
        if res.err != 0 {
            return Err(generated::types::Errno::Io.into());
        }
        Ok(generated::types::Fd::from(self.vfs.place_content(
            FileContentsUnevaluated::from_contents(vec_from_cstr_libc(res.str), 0),
        )))
    }

    fn call_llm(
        &mut self,
        mem: &mut wiggle::GuestMemory<'_>,
        config: wiggle::GuestPtr<str>,
        prompt: wiggle::GuestPtr<str>,
    ) -> Result<generated::types::Fd, generated::types::Error> {
        if self.context.data.conf.is_deterministic {
            return Err(generated::types::Errno::DeterministicViolation.into());
        }
        let config_str = read_string(mem, config)?;
        let config_str = CString::new(config_str).map_err(|e| generated::types::Errno::Inval)?;
        let prompt_str = read_string(mem, prompt)?;
        let prompt_str = CString::new(prompt_str).map_err(|e| generated::types::Errno::Inval)?;

        let supervisor = self.context.data.supervisor.clone();
        let Ok(mut supervisor) = supervisor.lock() else {
            return Err(generated::types::Errno::Io.into());
        };
        let mut fuel = self.context.shared_data.fuel_descriptor.get_fuel();
        let init_fuel = fuel;
        let res = supervisor.modules.llm.call_llm(
            &mut fuel,
            config_str.as_bytes().as_ptr(),
            prompt_str.as_bytes().as_ptr(),
        );
        self.context
            .shared_data
            .fuel_descriptor
            .consume_fuel(init_fuel - fuel);
        if res.err != 0 {
            return Err(generated::types::Errno::Io.into());
        }
        Ok(generated::types::Fd::from(self.vfs.place_content(
            FileContentsUnevaluated::from_contents(vec_from_cstr_libc(res.str), 0),
        )))
    }

    fn run_nondet(
        &mut self,
        mem: &mut wiggle::GuestMemory<'_>,
        eq_principle: wiggle::GuestPtr<str>,
        data: &generated::types::Bytes,
    ) -> Result<generated::types::Fd, generated::types::Error> {
        if !self.context.data.conf.can_spawn_nondet {
            return Err(generated::types::Errno::DeterministicViolation.into());
        }
        let eq_principle = read_string(mem, eq_principle)?;

        // relaxed reason: here is no actual race possible, only the determinsiitc vm can call it, and it has no concurrency
        let call_no = self
            .context
            .shared_data
            .nondet_call_no
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let cow = mem.as_cow(data.buf.as_array(data.buf_len))?;
        let mut entrypoint = Vec::from(b"nondet!");
        entrypoint.extend(cow.iter());
        let entrypoint = Arc::from(entrypoint);

        let supervisor = self.context.data.supervisor.clone();

        let essential_data = EssentialGenlayerSdkData {
            conf: base::Config {
                is_deterministic: false,
                can_read_storage: false,
                can_write_storage: false,
                can_spawn_nondet: false,
            },
            message_data: self.context.data.message_data.clone(),
            entrypoint,
            supervisor: supervisor.clone(),
            init_actions: self.context.data.init_actions.clone(),
        };

        let res = self
            .context
            .spaw_and_run(&supervisor, essential_data)
            .map_err(generated::types::Error::trap)?;

        let res: vm::RunResult = (|| {
            let Ok(mut supervisor) = supervisor.lock() else {
                return Err(anyhow::anyhow!("can't lock supervisor"));
            };
            let leader_res = supervisor.host.get_leader_result(call_no)?;

            match (leader_res, res) {
                (Some(vm::RunOk::Return(leader_res)), vm::RunOk::Return(res)) => {
                    // handle two null's
                    if leader_res == b"\x00" && res == b"\x00" {
                        Ok(vm::RunOk::Return(leader_res))
                    } else {
                        equivalence_principle_check(
                            &mut supervisor.host,
                            &eq_principle,
                            decode_nondet_return(&leader_res)?,
                            decode_nondet_return(&res)?,
                        )
                        .map(|_| vm::RunOk::Return(leader_res))
                    }
                }
                (Some(vm::RunOk::Rollback(leader_res)), vm::RunOk::Rollback(res)) => {
                    equivalence_principle_check(
                        &mut supervisor.host,
                        &eq_principle,
                        &leader_res,
                        &res,
                    )
                    .map(|_| vm::RunOk::Rollback(leader_res))
                }
                (None, res) => {
                    supervisor.host.post_result(call_no, &res)?;
                    Ok(res)
                }
                (_, _) => Err(anyhow::anyhow!("result diverged from leader's")),
            }
        })();

        self.set_vm_run_result(res.map_err(generated::types::Error::trap)?)
    }

    fn call_contract(
        &mut self,
        mem: &mut wiggle::GuestMemory<'_>,
        account: &generated::types::Addr,
        calldata: &generated::types::Bytes,
    ) -> Result<generated::types::Fd, generated::types::Error> {
        self.context.ensure_det()?;
        let called_contract_account = Address::read_from_mem(account, mem)?;
        let mut res_calldata = b"call!".to_vec();
        let calldata = calldata.buf.as_array(calldata.buf_len);
        res_calldata.extend(mem.as_cow(calldata)?.iter());
        let res_calldata = Arc::from(res_calldata);

        let supervisor = self.context.data.supervisor.clone();
        let init_actions = {
            let Ok(mut supervisor) = supervisor.lock() else {
                return Err(generated::types::Errno::Io.into());
            };
            supervisor
                .get_actions_for(&called_contract_account)
                .map_err(|_e| generated::types::Errno::Inval)
        }?;

        let my_conf = self.context.data.conf;

        let my_data = self.context.data.message_data.clone();

        let essential_data = EssentialGenlayerSdkData {
            conf: base::Config {
                is_deterministic: true,
                can_read_storage: my_conf.can_read_storage,
                can_write_storage: false,
                can_spawn_nondet: my_conf.can_spawn_nondet,
            },
            message_data: MessageData {
                contract_account: called_contract_account,
                sender_account: my_data.sender_account, // FIXME: is that true?
                gas: my_data.gas,                       // FIXME: is that true?
                value: None,
                is_init: false,
            },
            entrypoint: res_calldata,
            supervisor: supervisor.clone(),
            init_actions,
        };

        let res = self
            .context
            .spaw_and_run(&supervisor, essential_data)
            .map_err(generated::types::Error::trap)?;

        self.set_vm_run_result(res)
    }

    fn storage_read(
        &mut self,
        mem: &mut wiggle::GuestMemory<'_>,
        slot: &generated::types::Addr,
        index: u32,
        buf: &generated::types::MutBytes,
    ) -> Result<(), generated::types::Error> {
        if !self.context.data.conf.can_read_storage {
            return Err(generated::types::Errno::DeterministicViolation.into());
        }
        let dest_buf = buf.buf.as_array(buf.buf_len);

        let account = self.context.data.message_data.contract_account;

        let slot = Address::read_from_mem(&slot, mem)?;
        let mem_size = buf.buf_len as usize;
        let mut vec = Vec::with_capacity(mem_size);
        unsafe { vec.set_len(mem_size) };
        let supervisor = self.context.data.supervisor.clone();
        let Ok(mut supervisor) = supervisor.lock() else {
            return Err(generated::types::Errno::Io.into());
        };
        let mut fuel = self.context.shared_data.fuel_descriptor.get_fuel();
        let init_fuel = fuel;
        let res = supervisor
            .host
            .storage_read(&mut fuel, account, slot, index, &mut vec);
        self.context
            .shared_data
            .fuel_descriptor
            .consume_fuel(init_fuel - fuel);
        res.map_err(|_e| generated::types::Errno::Io)?;
        mem.copy_from_slice(&vec, dest_buf)?;
        Ok(())
    }

    fn storage_write(
        &mut self,
        mem: &mut wiggle::GuestMemory<'_>,
        slot: &generated::types::Addr,
        index: u32,
        buf: &generated::types::Bytes,
    ) -> Result<(), generated::types::Error> {
        if !self.context.data.conf.can_write_storage {
            return Err(generated::types::Errno::DeterministicViolation.into());
        }
        if !self.context.data.conf.can_read_storage {
            return Err(generated::types::Errno::DeterministicViolation.into());
        }

        let buf: Vec<u8> = buf.read_owned(mem)?;

        let account = self.context.data.message_data.contract_account;
        let slot = Address::read_from_mem(&slot, mem)?;

        let supervisor = self.context.data.supervisor.clone();
        let Ok(mut supervisor) = supervisor.lock() else {
            return Err(generated::types::Errno::Io.into());
        };
        let mut fuel = self.context.shared_data.fuel_descriptor.get_fuel();
        let init_fuel = fuel;
        let res = supervisor
            .host
            .storage_write(&mut fuel, account, slot, index, &buf);
        self.context
            .shared_data
            .fuel_descriptor
            .consume_fuel(init_fuel - fuel);
        res.map_err(|_e| generated::types::Errno::Io)?;
        Ok(())
    }
}

impl Context {
    pub fn log(&self, to: &mut dyn std::io::Write) {
        let _ = to.write_fmt(
            format_args!(
                "config {}\n",
                serde_json::json!({
                    "config": &self.data.conf,
                    "message": self.data.message_data
                })
            )
        );
    }

    /// note: handles fuel itself
    fn spaw_and_run(
        &mut self,
        supervisor: &Arc<Mutex<crate::vm::Supervisor>>,
        essential_data: EssentialGenlayerSdkData,
    ) -> vm::RunResult {
        let (mut vm, instance) = {
            let mut supervisor = supervisor
                .lock()
                .map_err(|_e| anyhow::anyhow!("can't lock supervisor"))?;
            let mut vm = supervisor.spawn(essential_data)?;
            let instance = supervisor.apply_actions(&mut vm)?;
            (vm, instance)
        };
        vm.run(&instance)
    }
}

fn vec_from_cstr_libc(str: *const u8) -> Arc<[u8]> {
    let res = Arc::from(unsafe { CStr::from_ptr(str as *const i8) }.to_bytes());
    unsafe {
        libc::free(str as *mut std::ffi::c_void);
    }
    res
}

fn decode_nondet_return<'a>(cur: &'a [u8]) -> Result<&'a str, anyhow::Error> {
    if cur.is_empty() {
        anyhow::bail!("invalid nondet return ; expected calldata encoded string; got empty")
    }

    let mut len: u64 = 0u64;
    let mut off = 0u64;

    let mut idx = 0usize;
    while idx < cur.len() {
        let byte = cur[idx];
        idx += 1;
        len |= (byte as u64 & 0x7f) << off;
        off += 7;
        if byte & 0x80 == 0 {
            break;
        }
        if off >= 40 {
            anyhow::bail!("invalid nondet return ; string length is too big")
        }
    }
    let typ = len & 0x7;
    len >>= 3;
    if typ != 5 {
        anyhow::bail!("invalid nondet return ; expected string")
    }
    if len > u32::max_value() as u64 {
        anyhow::bail!("invalid nondet return ; string length is too big")
    }
    let len = len as u32;
    if idx + len as usize != cur.len() {
        anyhow::bail!("invalid nondet return ; string size is encoded incorrectly")
    }
    Ok(str::from_utf8(&cur[idx..])?)
}

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
enum EqPrincipleMode {
    Refl,
}

#[derive(Deserialize)]
struct EqPrincipleConfig {
    mode: EqPrincipleMode,
}

fn equivalence_principle_check(
    _host: &mut Host,
    config: &str,
    leader: &str,
    cur: &str,
) -> Result<(), anyhow::Error> {
    let config: EqPrincipleConfig = serde_json::from_str(config)?;
    let is_ok = match config.mode {
        EqPrincipleMode::Refl => leader == cur,
    };
    if !is_ok {
        anyhow::bail!("equivalence_principle_check failed");
    }
    Ok(())
}
