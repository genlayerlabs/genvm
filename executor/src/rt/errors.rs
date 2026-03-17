use std::collections::BTreeMap;

use crate::rt;
use genlayer_sdk::abi;
use genvm_common::*;

#[derive(Debug)]
pub struct VMError(pub abi::consts::VmError, pub Option<anyhow::Error>);

impl std::error::Error for VMError {}

impl std::fmt::Display for VMError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "VMError({})", self.0 .0)
    }
}

impl VMError {
    pub fn wrap<E: Into<anyhow::Error>>(message: abi::consts::VmError, cause: E) -> Self {
        match cause.into().downcast::<VMError>() {
            Err(cause) => Self(message, Some(cause)),
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

pub fn unwrap_vm_errors_fingerprint(
    err: UnwrapDynError,
) -> anyhow::Result<(rt::vm::RunOk, Fingerprint)> {
    let err = UnwrapDynError::from(err);

    let mut fingerprint = Fingerprint {
        frames: Vec::new(),
        module_instances: BTreeMap::new(),
    };

    if let Some(bt) = err.downcast_ref::<wasmtime::WasmBacktrace>() {
        let frames = bt
            .frames()
            .iter()
            .map(|f| Frame {
                module_name: f.module().name().unwrap_or("").to_string(),
                func: f.func_index(),
            })
            .collect();

        fingerprint.frames = frames;
    } else {
        log_warn!("no backtrace attached");
    }
    if let Some(fp) = err.downcast_ref::<wasmtime::Fingerprint>() {
        fingerprint.module_instances = fp.module_instances.clone();
    } else {
        log_warn!("no memories attached");
    }

    log_debug!(fp:serde = fingerprint, frames = fingerprint.frames.len(); "captured fingerprint");

    Ok((unwrap_vm_errors(err)?, fingerprint))
}

#[derive(Debug)]
pub struct UserError(pub calldata::Value);

impl std::error::Error for UserError {}

impl std::fmt::Display for UserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "UserError({:?})", self.0)
    }
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct Frame {
    pub module_name: String,
    pub func: u32,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SingleMemoryFP(#[serde(with = "serde_bytes")] pub [u8; 32]);

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub struct Fingerprint {
    pub frames: Vec<Frame>,

    pub module_instances: BTreeMap<String, wasmtime::ModuleFingerprint>,
}
