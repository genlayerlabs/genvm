use std::sync::Arc;

use wasmtime::StoreContextMut;
use wiggle::GuestError;

use super::base;

pub struct EssentialGenlayerSdkData {
    pub conf: base::Config,
    pub message_data: crate::node_iface::MessageData,
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

        fn build<'a>(&self) -> impl Fn(wasmtime::StoreContextMut<'a,T>) -> Self::MappedTo<'a> {
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
pub struct ContractReturn(pub String);

impl std::error::Error for ContractReturn {}

impl std::fmt::Display for ContractReturn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Rolled back with {}", self.0)
    }
}

impl ContextData {
    fn ensure_det(&self) -> Result<(), generated::types::Error> {
        if self.data.conf.is_deterministic {
            Err(generated::types::Errno::DeterministicViolation.into())
        } else {
            Ok(())
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
        message: wiggle::GuestPtr<str>,
    ) -> anyhow::Error {
        match super::common::read_string(mem, message) {
            Err(e) => e.into(),
            Ok(str) => ContractReturn(str).into(),
        }
    }

    fn run_nondet(
        &mut self,
        mem: &mut wiggle::GuestMemory<'_>,
        eq_principle: wiggle::GuestPtr<str>,
        data: &generated::types::Bytes,
    ) -> Result<generated::types::BytesLen, generated::types::Error> {
        todo!()
    }
}
