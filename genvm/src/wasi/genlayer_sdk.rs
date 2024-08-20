use wiggle::GuestError;

use super::base;

pub struct EssentialGenlayerSdkData {
    pub conf: base::Config,
    pub message_data: crate::node_iface::MessageData,
}

pub struct Context {
    pub data: EssentialGenlayerSdkData,

    result: Vec<u8>,
    result_cursor: usize,
}

pub(crate) mod generated {
    wiggle::from_witx!({
        witx: ["$CARGO_MANIFEST_DIR/src/wasi/witx/genlayer_sdk.witx"],
        errors: { errno => trappable Error },
    });

    wiggle::wasmtime_integration!({
        witx: ["$CARGO_MANIFEST_DIR/src/wasi/witx/genlayer_sdk.witx"],
        errors: { errno => trappable Error },
        target: self,
    });
}

impl Context {
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

pub(super) fn add_to_linker_sync<T: Send>(
    linker: &mut wasmtime::Linker<T>,
    f: impl Fn(&mut T) -> &mut Context + Copy + Send + Sync + 'static,
) -> anyhow::Result<()> {
    generated::add_genlayer_sdk_to_linker(linker, f)?;
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

impl Context {
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

#[allow(unused_variables)]
impl generated::genlayer_sdk::GenlayerSdk for Context {
    fn get_calldata(&mut self,mem: &mut wiggle::GuestMemory<'_>) -> Result<generated::types::BytesLen, generated::types::Error> {
        self.result = Vec::from(self.data.message_data.calldata.as_str());
        self.result_cursor = 0;
        let res = self.result.len().try_into()?;
        Ok(res)
    }

    fn read_result(&mut self,mem: &mut wiggle::GuestMemory<'_> ,buf:wiggle::GuestPtr<u8> ,len:u32) -> Result<generated::types::BytesLen,generated::types::Error> {
        self.ensure_det()?;

        let len_left = self.result.len() - self.result_cursor;
        let len_left = len_left.min(len as usize);
        let len_left_u32  = len_left.try_into()?;

        mem.copy_from_slice(&self.result[self.result_cursor..self.result_cursor+len_left], buf.as_array(len_left_u32))?;
        self.result_cursor += len_left;

        Ok(len_left_u32)
    }

    fn rollback(&mut self,mem: &mut wiggle::GuestMemory<'_> ,message:wiggle::GuestPtr<str>) -> anyhow::Error {
        match super::common::read_string(mem, message) {
            Err(e) => e.into(),
            Ok(str) => Rollback(str).into()
        }
    }
}
