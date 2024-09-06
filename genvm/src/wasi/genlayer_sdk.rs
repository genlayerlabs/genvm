use std::{ffi::{CStr, CString}, sync::{Arc, Mutex}};

use wasmtime::StoreContextMut;
use wiggle::GuestError;

use crate::{node_iface::{self, Address, MessageData, StorageSlot}, vm::InitActions};

use super::{base, common::read_string};

pub struct EssentialGenlayerSdkData {
    pub conf: base::Config,
    pub message_data: crate::node_iface::MessageData,
    pub entrypoint: Vec<u8>,
    pub supervisor: Arc<Mutex<crate::vm::Supervisor>>,
    pub init_actions: InitActions,
}

pub struct ContextData {
    pub data: EssentialGenlayerSdkData,

    result: Vec<u8>,
    result_cursor: usize,
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

impl node_iface::Address {
    fn read_from_mem(addr: &generated::types::Addr, mem: &mut wiggle::GuestMemory<'_>) -> Result<Self, generated::types::Error> {
        let cow = mem.as_cow(addr.ptr.as_array(node_iface::Address::len().try_into().unwrap()))?;
        let mut ret = Address::new();
        for (x, y) in ret.0.iter_mut().zip(cow.iter()) {
            *x = *y;
        }
        Ok(ret)
    }
}

impl generated::types::Bytes {
    #[allow(dead_code)]
    fn read_owned(&self, mem: &mut wiggle::GuestMemory<'_>) -> Result<Vec<u8>, generated::types::Error> {
        Ok(mem.as_cow(self.buf.as_array(self.buf_len))?.into_owned())
    }
}

impl ContextData {
    pub fn new(data: EssentialGenlayerSdkData) -> Self {
        Self {
            data,
            result: Vec::new(),
            result_cursor: 0,
        }
    }
}

impl wiggle::GuestErrorType for generated::types::Errno {
    fn success() -> Self {
        Self::Success
    }
}

trait MappedVtable<T> {
    fn get_data_mut<'a, 'b>(&self, store: &'b mut StoreContextMut<'a, T>) -> &'b mut ContextData;
}

struct Mapped<'a, T> {
    stor: StoreContextMut<'a, T>,
    vtable: Arc<dyn MappedVtable<T>>,
}

impl<'a, T> Mapped<'a, T> {
    fn data_mut<'loc>(&'loc mut self) -> &'loc mut ContextData {
        self.vtable.get_data_mut(&mut self.stor)
    }

    fn read<'loc, R>(&'loc mut self, f: impl Fn(&ContextData) -> R) -> R {
        let data: &ContextData = self.vtable.get_data_mut(&mut self.stor);
        f(data)
    }
}

pub(super) fn add_to_linker_sync<'a, T: Send + 'static, F: Fn(&mut T) -> &mut ContextData + Copy + Send + Sync + 'static>(
    linker: &mut wasmtime::Linker<T>,
    f: F,
) -> anyhow::Result<()> {
    struct A<F: Send + Sync> {
        f: F,
    }
    impl<T, F: Fn(&mut T) -> &mut ContextData + Copy + Send + Sync + 'static> MappedVtable<T> for A<F> {
        fn get_data_mut<'a, 'b>(&self, store: &'b mut StoreContextMut<'a, T>) -> &'b mut ContextData {
            let fc = &self.f;
            fc(store.data_mut())
        }
    }

    let vtable = Arc::new(A::<F> {
        f,
    });

    //#[derive(Send, Sync)]
    struct FnBuilderImpl<T> {
        vtable: Arc<dyn MappedVtable<T> + Send + Sync + 'static>
    }
    impl<T> Clone for FnBuilderImpl<T> {
        fn clone(&self) -> Self {
            Self { vtable: self.vtable.clone() }
        }
    }
    impl<T: 'static> generated::FnBuilderGenlayerSdk<T> for FnBuilderImpl<T> {
        type MappedTo<'a> = Mapped<'a, T>;

        fn build<'a>(&self) -> impl Fn(wasmtime::StoreContextMut<'a,T>) -> Mapped<'a, T> {
            |x| {
                Mapped {
                    stor: x,
                    vtable: self.vtable.clone(),
                }
            }
        }
    }

    let fn_bilder = FnBuilderImpl {
        vtable: vtable,
    };

    generated::add_genlayer_sdk_to_linker(linker, fn_bilder)?;
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

impl ContextData {
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

impl<'a, T> Mapped<'a, T> {
    fn consume_fuel(&mut self, gas_consumed: u64) -> Result<(), generated::types::Error> {
        let old_fuel = self.stor.get_fuel().map_err(|_e| generated::types::Errno::Io)?;
        let gas_consumed = gas_consumed.min(old_fuel).max(1);
        self.stor.set_fuel(old_fuel - gas_consumed).map_err(|_e| generated::types::Errno::Io)?;
        Ok(())
    }

    fn set_result(&mut self, data: Vec<u8>) -> Result<generated::types::BytesLen, generated::types::Error> {
        self.data_mut().result = data;
        self.data_mut().result_cursor = 0;
        let res: u32 = self.data_mut().result.len().try_into()?;
        let mut gas_consumed: u64 = res.into();
        gas_consumed /= 32;
        self.consume_fuel(gas_consumed)?;
        Ok(res)
    }
}

#[allow(unused_variables)]
impl<'a, T> generated::genlayer_sdk::GenlayerSdk for Mapped<'a, T> {
    fn get_message_data(
        &mut self,
        mem: &mut wiggle::GuestMemory<'_>,
    ) -> Result<generated::types::BytesLen, generated::types::Error> {
        let res = serde_json::to_string(&self.data_mut().data.message_data)?;
        self.set_result(Vec::from(res))
    }

    fn get_entrypoint(
        &mut self,
        mem: &mut wiggle::GuestMemory<'_>,
    ) -> Result<generated::types::BytesLen, generated::types::Error> {
        let ep = self.data_mut().data.entrypoint.clone();
        self.set_result(ep)
    }

    fn read_result(
        &mut self,
        mem: &mut wiggle::GuestMemory<'_>,
        buf: wiggle::GuestPtr<u8>,
        len: u32,
    ) -> Result<generated::types::BytesLen, generated::types::Error> {
        let cursor = self.read(|x| x.result_cursor);
        let len_left = self.data_mut().result.len() - cursor;
        let len_left = len_left.min(len as usize);
        let len_left_u32 = len_left.try_into()?;

        mem.copy_from_slice(
            &self.data_mut().result[cursor..cursor + len_left],
            buf.as_array(len_left_u32),
        )?;
        self.data_mut().result_cursor += len_left;

        Ok(len_left_u32)
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
        let Ok(res) = res else { return res.unwrap_err().into(); };
        ContractReturn(res).into()
    }

    fn get_webpage(
        &mut self,
        mem: &mut wiggle::GuestMemory<'_>,
        config: wiggle::GuestPtr<str>,
        url: wiggle::GuestPtr<str>,
    ) -> Result<generated::types::BytesLen, generated::types::Error> {
        if self.data_mut().data.conf.is_deterministic {
            return Err(generated::types::Errno::DeterministicViolation.into());
        }
        let config_str = read_string(mem, config)?;
        let config_str = CString::new(config_str).map_err(|e| generated::types::Errno::Inval)?;
        let url_str = read_string(mem, url)?;
        let url_str = CString::new(url_str).map_err(|e| generated::types::Errno::Inval)?;

        let supervisor = self.data_mut().data.supervisor.clone();
        let Ok(mut supervisor) = supervisor.lock() else { return Err(generated::types::Errno::Io.into()); };
        let mut fuel = self.stor.get_fuel().map_err(|_e| generated::types::Errno::Io)?;
        let res = supervisor.api.get_webpage(&mut fuel, config_str.as_bytes().as_ptr(), url_str.as_bytes().as_ptr());
        self.stor.set_fuel(fuel).map_err(|_e| generated::types::Errno::Io)?;
        if res.err != 0 {
            return Err(generated::types::Errno::Io.into());
        }
        self.set_result(vec_from_cstr_libc(res.str))
    }

    fn call_llm(
        &mut self,
        mem: &mut wiggle::GuestMemory<'_>,
        config: wiggle::GuestPtr<str>,
        prompt: wiggle::GuestPtr<str>,
    ) -> Result<generated::types::BytesLen, generated::types::Error> {
        if self.data_mut().data.conf.is_deterministic {
            return Err(generated::types::Errno::DeterministicViolation.into());
        }
        let config_str = read_string(mem, config)?;
        let config_str = CString::new(config_str).map_err(|e| generated::types::Errno::Inval)?;
        let prompt_str = read_string(mem, prompt)?;
        let prompt_str = CString::new(prompt_str).map_err(|e| generated::types::Errno::Inval)?;

        let supervisor = self.data_mut().data.supervisor.clone();
        let Ok(mut supervisor) = supervisor.lock() else { return Err(generated::types::Errno::Io.into()); };
        let mut fuel = self.stor.get_fuel().map_err(|_e| generated::types::Errno::Io)?;
        let res = supervisor.api.call_llm(&mut fuel, config_str.as_bytes().as_ptr(), prompt_str.as_bytes().as_ptr());
        self.stor.set_fuel(fuel).map_err(|_e| generated::types::Errno::Io)?;
        if res.err != 0 {
            return Err(generated::types::Errno::Io.into());
        }
        self.set_result(vec_from_cstr_libc(res.str))
    }

    fn run_nondet(
        &mut self,
        mem: &mut wiggle::GuestMemory<'_>,
        eq_principle: wiggle::GuestPtr<str>,
        data: &generated::types::Bytes,
    ) -> Result<generated::types::BytesLen, generated::types::Error> {
        if !self.data_mut().data.conf.can_spawn_nondet {
            return Err(generated::types::Errno::DeterministicViolation.into());
        }

        let cow = mem.as_cow(data.buf.as_array(data.buf_len))?;
        let mut entrypoint = Vec::from(b"nondet!");
        entrypoint.extend(cow.iter());

        let supervisor = self.data_mut().data.supervisor.clone();

        let essential_data = EssentialGenlayerSdkData {
            conf: base::Config {
                is_deterministic: false,
                can_read_storage: false,
                can_write_storage: false,
                can_spawn_nondet: false,
            },
            message_data: self.data_mut().data.message_data.clone(),
            entrypoint,
            supervisor: supervisor.clone(),
            init_actions: self.data_mut().data.init_actions.clone(),
        };

        let res = self.spaw_and_run(supervisor, essential_data);

        //let res = eq_principle
        let res = res.map_err(|_e| generated::types::Errno::Io);

        match res? {
            crate::vm::VMRunResult::Return(r) => {
                self.set_result(r)
            },
            crate::vm::VMRunResult::Rollback(r) => Err(generated::types::Error::trap(Rollback(r).into())),
            crate::vm::VMRunResult::Error(e) => Err(generated::types::Error::trap(Rollback(format!("subvm failed {}", e)).into())),
        }
    }

    fn call_contract(
        &mut self,
        mem: &mut wiggle::GuestMemory<'_>,
        account: &generated::types::Addr,
        calldata: &generated::types::Bytes,
    ) -> Result<generated::types::BytesLen, generated::types::Error> {
        self.data_mut().ensure_det()?;
        let called_contract_account = Address::read_from_mem(account, mem)?;
        let mut res_calldata = b"call!".to_vec();
        let calldata = calldata.buf.as_array(calldata.buf_len);
        res_calldata.extend(mem.as_cow(calldata)?.iter());

        let supervisor = self.data_mut().data.supervisor.clone();
        let init_actions = {
            let Ok(mut supervisor) = supervisor.lock() else { return Err(generated::types::Errno::Io.into()); };
            supervisor.get_actions_for(&called_contract_account).map_err(|_e| generated::types::Errno::Inval)
        }?;

        let my_conf = self.data_mut().data.conf;

        let my_data = self.data_mut().data.message_data.clone();

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
                gas: my_data.gas, // FIXME: is that true?
                value: None,
                is_init: false,
            },
            entrypoint: res_calldata,
            supervisor: supervisor.clone(),
            init_actions,
        };

        let res = self.spaw_and_run(supervisor, essential_data);
        let res = res.map_err(|_e| generated::types::Errno::Io);

        match res? {
            crate::vm::VMRunResult::Return(r) => {
                self.set_result(r)
            },
            crate::vm::VMRunResult::Rollback(r) => Err(generated::types::Error::trap(Rollback(r).into())),
            crate::vm::VMRunResult::Error(e) => Err(generated::types::Error::trap(Rollback(format!("subvm failed {}", e)).into())),
        }
    }

    fn storage_read(&mut self,mem: &mut wiggle::GuestMemory<'_> , slot: &generated::types::Addr,index: u32, buf: &generated::types::MutBytes) -> Result<(), generated::types::Error>  {
        if !self.data_mut().data.conf.can_read_storage {
            return Err(generated::types::Errno::DeterministicViolation.into());
        }
        let dest_buf = buf.buf.as_array(buf.buf_len);

        let account = self.data_mut().data.message_data.contract_account;

        let slot = Address::read_from_mem(&slot, mem)?;
        let mem_size = buf.buf_len as usize;
        let mut vec = Vec::with_capacity(mem_size);
        unsafe { vec.set_len(mem_size) };
        let supervisor = self.data_mut().data.supervisor.clone();
        let Ok(mut supervisor) = supervisor.lock() else { return Err(generated::types::Errno::Io.into()); };
        let mut rem_gas = node_iface::Gas(self.stor.get_fuel().map_err(|_e| generated::types::Errno::Io)?);
        let res = supervisor.api.storage_read(&mut rem_gas, StorageSlot {account, slot}, index, &mut vec);
        let _ = self.stor.set_fuel(rem_gas.raw());
        res.map_err(|_e| generated::types::Errno::Io)?;
        mem.copy_from_slice(&vec, dest_buf)?;
        Ok(())
    }

    fn storage_write(&mut self,mem: &mut wiggle::GuestMemory<'_>, slot:&generated::types::Addr,index: u32, buf: &generated::types::Bytes) -> Result<(), generated::types::Error>  {
        if !self.data_mut().data.conf.can_write_storage {
            return Err(generated::types::Errno::DeterministicViolation.into());
        }
        if !self.data_mut().data.conf.can_read_storage {
            return Err(generated::types::Errno::DeterministicViolation.into());
        }

        let buf: Vec<u8> = buf.read_owned(mem)?;

        let account = self.data_mut().data.message_data.contract_account;
        let slot = Address::read_from_mem(&slot, mem)?;

        let supervisor = self.data_mut().data.supervisor.clone();
        let Ok(mut supervisor) = supervisor.lock() else { return Err(generated::types::Errno::Io.into()); };
        let mut rem_gas = node_iface::Gas(self.stor.get_fuel().map_err(|_e| generated::types::Errno::Io)?);
        let res = supervisor.api.storage_write(&mut rem_gas, StorageSlot {account, slot}, index, &buf);
        let _ = self.stor.set_fuel(rem_gas.raw());
        res.map_err(|_e| generated::types::Errno::Io)?;
        Ok(())
    }
}

impl<T> Mapped<'_, T> {
    fn spaw_and_run(&mut self, supervisor: Arc<Mutex<crate::vm::Supervisor>>, essential_data: EssentialGenlayerSdkData) -> Result<crate::vm::VMRunResult, ()> {
        fn dummy_error<E>(_e: E) -> () { () }
        let (mut vm, instance) = {
            let mut supervisor = supervisor.lock().map_err(dummy_error)?;
            let mut vm = supervisor.spawn(essential_data).map_err(dummy_error)?;
            let instance = supervisor.apply_actions(&mut vm).map_err(dummy_error)?;
            (vm, instance)
        };

        let pre_fuel = self.stor.get_fuel().map_err(dummy_error)?;
        vm.store.set_fuel(pre_fuel).map_err(dummy_error)?;

        let res = vm.run(&instance).map_err(dummy_error);

        let remaining_fuel = vm.store.get_fuel().unwrap_or(0);
        let _ = self.stor.set_fuel(remaining_fuel);

        res
    }
    //EssentialGenlayerSdkData
}

fn vec_from_cstr_libc(str: *const u8) -> Vec<u8> {
    let res = Vec::from(unsafe  { CStr::from_ptr(str as *const i8) }.to_bytes());
    unsafe { libc::free(str as *mut std::ffi::c_void); }
    res
}
