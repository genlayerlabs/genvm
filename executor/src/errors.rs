use std::collections::BTreeMap;

use crate::{public_abi, vm};
use genvm_common::*;

#[derive(Debug)]
pub struct VMError(pub String, pub Option<anyhow::Error>);

impl std::error::Error for VMError {}

impl std::fmt::Display for VMError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "VMError({})", self.0)
    }
}

impl VMError {
    pub fn oom(cause: Option<anyhow::Error>) -> Self {
        VMError(public_abi::VmError::Oom.value().into(), cause)
    }

    pub fn wrap(message: String, cause: anyhow::Error) -> Self {
        match cause.downcast::<VMError>() {
            Err(cause) => Self(message, Some(cause)),
            Ok(v) => v,
        }
    }
}

#[allow(clippy::manual_try_fold)]
pub fn unwrap_vm_errors(err: anyhow::Error) -> anyhow::Result<vm::RunOk> {
    let res: anyhow::Result<vm::RunOk> = [
        |e: anyhow::Error| match e.downcast::<crate::wasi::preview1::I32Exit>() {
            Ok(crate::wasi::preview1::I32Exit(0)) => Ok(vm::RunOk::empty_return()),
            Ok(crate::wasi::preview1::I32Exit(v)) => {
                Ok(vm::RunOk::VMError(format!("exit_code {v}"), None))
            }
            Err(e) => Err(e),
        },
        |e: anyhow::Error| {
            e.downcast::<wasmtime::Trap>()
                .map(|v| vm::RunOk::VMError(format!("wasm_trap {v:?}"), Some(v.into())))
        },
        |e: anyhow::Error| {
            e.downcast::<crate::errors::VMError>()
                .map(|crate::errors::VMError(m, c)| vm::RunOk::VMError(m, c))
        },
        |e: anyhow::Error| {
            e.downcast::<crate::errors::UserError>()
                .map(|crate::errors::UserError(v)| vm::RunOk::UserError(v))
        },
        |e: anyhow::Error| {
            e.downcast::<crate::wasi::genlayer_sdk::ContractReturn>()
                .map(|crate::wasi::genlayer_sdk::ContractReturn(v)| vm::RunOk::Return(v))
        },
    ]
    .into_iter()
    .fold(Err(err), |acc, func| match acc {
        Ok(acc) => Ok(acc),
        Err(e) => func(e),
    });

    res
}

pub fn unwrap_vm_errors_fingerprint(
    err: anyhow::Error,
) -> anyhow::Result<(vm::RunOk, Fingerprint)> {
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

    log_debug!(fp:serde = fingerprint; "captured fingerprint");

    Ok((unwrap_vm_errors(err)?, fingerprint))
}

#[derive(Debug)]
pub struct UserError(pub String);

impl std::error::Error for UserError {}

impl std::fmt::Display for UserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "UserError({:?})", self.0)
    }
}

#[derive(Debug, serde::Serialize)]
pub struct Frame {
    pub module_name: String,
    pub func: u32,
}

#[derive(Debug, serde::Serialize)]
pub struct SingleMemoryFP(#[serde(with = "serde_bytes")] pub [u8; 32]);

#[derive(Debug, serde::Serialize)]
pub struct Fingerprint {
    pub frames: Vec<Frame>,

    pub module_instances: BTreeMap<String, wasmtime::ModuleFingerprint>,
}
