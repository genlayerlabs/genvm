use std::collections::BTreeMap;

use crate::rt;
use genlayer_sdk::abi;
use genvm_common::*;

#[derive(Debug)]
pub struct VMError(pub abi::consts::VmError, pub Option<anyhow::Error>);

impl std::error::Error for VMError {}

impl std::fmt::Display for VMError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if f.alternate() {
            f.write_fmt(format_args!("VMError({},", self.0 .0))?;
            if let Some(cause) = &self.1 {
                f.write_fmt(format_args!("{})", cause))?;
            } else {
                f.write_str("None)")?;
                return Ok(());
            };
        } else {
            f.write_fmt(format_args!("VMError({})", self.0 .0))?;
        }

        Ok(())
    }
}

impl VMError {
    pub fn wrap<E: Into<anyhow::Error>>(message: abi::consts::VmError, cause: E) -> Self {
        match cause.into().downcast::<VMError>() {
            Err(cause) => {
                log_debug!(vm_error = message.0, cause:ah = cause; "wrapping VMError");
                Self(message, Some(cause))
            }
            Ok(v) => v,
        }
    }
}

#[allow(clippy::manual_try_fold)]
pub fn unwrap_vm_errors(err: UnwrapDynError) -> anyhow::Result<rt::vm::RunOk> {
    let res: std::result::Result<rt::vm::RunOk, UnwrapDynError> = [
        |e: UnwrapDynError| match e.downcast::<crate::wasi::preview1::I32Exit>() {
            Ok(crate::wasi::preview1::I32Exit(0)) => Ok(rt::vm::RunOk::empty_return()),
            Ok(crate::wasi::preview1::I32Exit(v)) => Ok(rt::vm::RunOk::VMError(
                abi::consts::VmError::exit_code().val_i32(v),
                None,
            )),
            Err(e) => Err(e),
        },
        |e: UnwrapDynError| {
            e.downcast::<wasmtime::Trap>().map(|v| {
                rt::vm::RunOk::VMError(
                    abi::consts::VmError::wasm_trap().val_str(&format!("{v:?}")),
                    Some(v.into()),
                )
            })
        },
        |e: UnwrapDynError| {
            e.downcast::<wiggle::GuestError>().map(|e| {
                rt::vm::RunOk::VMError(
                    abi::consts::VmError::wasm_trap().val_str("fault"),
                    Some(e.into()),
                )
            })
        },
        |e: UnwrapDynError| {
            e.downcast::<rt::errors::VMError>()
                .map(|rt::errors::VMError(m, c)| rt::vm::RunOk::VMError(m, c))
        },
        |e: UnwrapDynError| {
            e.downcast::<rt::errors::UserError>()
                .map(|rt::errors::UserError(v)| rt::vm::RunOk::UserError(v))
        },
        |e: UnwrapDynError| {
            e.downcast::<crate::wasi::genlayer_sdk::ContractReturn>()
                .map(|crate::wasi::genlayer_sdk::ContractReturn(v)| rt::vm::RunOk::Return(v))
        },
    ]
    .into_iter()
    .fold(Err(err), |acc, func| match acc {
        Ok(acc) => Ok(acc),
        Err(e) => func(e),
    });

    match res {
        Ok(r) => Ok(r),
        Err(UnwrapDynError::Anyhow(e)) => Err(e),
        Err(UnwrapDynError::Wasmtime(e)) => Err(crate::wasmtime_to_anyhow(e)),
    }
}

pub enum UnwrapDynError {
    Anyhow(anyhow::Error),
    Wasmtime(wasmtime::Error),
}

impl From<anyhow::Error> for UnwrapDynError {
    fn from(err: anyhow::Error) -> Self {
        match err.downcast::<wasmtime::Error>() {
            Ok(casted) => From::<wasmtime::Error>::from(casted),
            Err(e) => UnwrapDynError::Anyhow(e),
        }
    }
}

impl From<wasmtime::Error> for UnwrapDynError {
    fn from(err: wasmtime::Error) -> Self {
        match err.downcast::<anyhow::Error>() {
            Ok(casted) => From::<anyhow::Error>::from(casted),
            Err(e) => UnwrapDynError::Wasmtime(e),
        }
    }
}

impl UnwrapDynError {
    fn downcast_ref<E>(&self) -> Option<&E>
    where
        E: std::fmt::Display + std::fmt::Debug + Send + Sync + 'static,
    {
        match self {
            UnwrapDynError::Anyhow(e) => e.downcast_ref::<E>(),
            UnwrapDynError::Wasmtime(e) => e.downcast_ref::<E>(),
        }
    }

    fn downcast<E>(self) -> std::result::Result<E, Self>
    where
        E: std::fmt::Display + std::fmt::Debug + Send + Sync + 'static,
    {
        match self {
            UnwrapDynError::Anyhow(e) => e.downcast::<E>().map_err(UnwrapDynError::Anyhow),
            UnwrapDynError::Wasmtime(e) => e.downcast::<E>().map_err(UnwrapDynError::Wasmtime),
        }
    }
}

/// Recover the call stack carried by a trapping error, if any. This is
/// independent of the wasm store and can be done even after it is consumed.
pub fn extract_backtrace(err: &UnwrapDynError) -> Option<Backtrace> {
    let Some(bt) = err.downcast_ref::<wasmtime::WasmBacktrace>() else {
        log_warn!("no backtrace attached");
        return None;
    };

    let frames = bt
        .frames()
        .iter()
        .map(|f| Frame {
            module_name: f.module().name().unwrap_or("").to_string(),
            func: f.func_index(),
        })
        .collect();

    Some(Backtrace { frames })
}

pub fn unwrap_vm_errors_backtrace(
    err: UnwrapDynError,
) -> anyhow::Result<(rt::vm::RunOk, Option<Backtrace>)> {
    let backtrace = extract_backtrace(&err);

    log_debug!(
        bt:serde = backtrace,
        frames = backtrace.as_ref().map_or(0, |b| b.frames.len());
        "captured backtrace"
    );

    Ok((unwrap_vm_errors(err)?, backtrace))
}

#[derive(Debug)]
pub struct UserError(pub calldata::unparsed::Maybe<calldata::Value>);

impl std::error::Error for UserError {}

impl std::fmt::Display for UserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "UserError({:?})", self.0)
    }
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq, genlayer_calldata::Encode)]
pub struct Frame {
    pub module_name: String,
    pub func: u32,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SingleMemoryFP(#[serde(with = "serde_bytes")] pub [u8; 32]);

impl<W: calldata::Writer> calldata::codec::Encode<W> for SingleMemoryFP {
    type Error = W::Error;

    fn encode(&self, enc: &mut calldata::Encoder<W>) -> Result<(), Self::Error> {
        enc.push_bytes(&self.0)
    }
}

/// The wasm call stack captured at the point of a trap. Carried by the error
/// itself, so it survives even when the store is gone.
#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct Backtrace {
    pub frames: Vec<Frame>,
}

impl<W: calldata::Writer> calldata::codec::Encode<W> for Backtrace {
    type Error = W::Error;

    fn encode(&self, enc: &mut calldata::Encoder<W>) -> Result<(), Self::Error> {
        calldata::codec::Encode::encode(&self.frames, enc)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WasmStoreHashes(pub BTreeMap<String, wasmtime::ModuleFingerprint>);

impl<W: calldata::Writer> calldata::codec::Encode<W> for WasmStoreHashes {
    type Error = W::Error;

    fn encode(&self, enc: &mut calldata::Encoder<W>) -> Result<(), Self::Error> {
        enc.start_map(self.0.len() as u64)?;
        for (k, v) in &self.0 {
            enc.push_map_k(k)?;
            encode_module_fingerprint(v, enc)?;
        }
        Ok(())
    }
}

fn encode_module_fingerprint<W: calldata::Writer>(
    mfp: &wasmtime::ModuleFingerprint,
    enc: &mut calldata::Encoder<W>,
) -> Result<(), W::Error> {
    // ModuleFingerprint has a single field: memories: Vec<MemoryFingerprint>
    enc.start_map(1)?;
    enc.push_map_k("memories")?;
    enc.start_array(mfp.memories.len() as u64)?;
    for mem in &mfp.memories {
        enc.push_bytes(&mem.0)?;
    }
    Ok(())
}
