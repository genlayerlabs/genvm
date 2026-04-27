use crate::{public_abi, rt, wasi};

use genlayer_sdk::abi;
use genvm_common::*;
use itertools::Itertools;

pub mod storage;

#[derive(serde::Serialize, Debug)]
pub enum RunOk {
    Return(Vec<u8>),
    UserError(calldata::Value),
    VMError(
        abi::consts::VmError,
        #[serde(skip_serializing)] Option<anyhow::Error>,
    ),
}

pub struct RunResult {
    pub run_ok: RunOk,
    pub fingerprint: Option<rt::errors::Fingerprint>,
    pub vm_data: wasi::genlayer_sdk::SingleVMData,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FullResult {
    pub kind: public_abi::ResultCode,
    pub data: calldata::Value,
    pub fingerprint: Option<rt::errors::Fingerprint>,
    pub storage_changes: Vec<storage::Delta>,

    pub emissions: Vec<genlayer_sdk::abi::ExecutionEmission>,
}

impl FullResult {
    pub fn empty_from(run_ok: RunOk) -> Self {
        Self {
            kind: match run_ok {
                RunOk::Return(_) => public_abi::ResultCode::Return,
                RunOk::UserError(_) => public_abi::ResultCode::UserError,
                RunOk::VMError(_, _) => public_abi::ResultCode::VmError,
            },
            data: match run_ok {
                RunOk::Return(buf) => calldata::Value::Bytes(buf),
                RunOk::UserError(val) => val,
                RunOk::VMError(msg, _) => calldata::Value::Str(msg.into()),
            },
            fingerprint: None,
            storage_changes: Vec::new(),
            emissions: Vec::new(),
        }
    }

    pub fn timeout() -> Self {
        Self {
            kind: public_abi::ResultCode::VmError,
            data: calldata::Value::Str(public_abi::VmError::timeout().into()),
            fingerprint: None,
            storage_changes: Vec::new(),
            emissions: Vec::new(),
        }
    }
}

impl RunOk {
    pub fn empty_return() -> Self {
        Self::Return([0].into())
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        use crate::public_abi::ResultCode;
        match self {
            RunOk::Return(buf) => {
                let mut res = Vec::with_capacity(1 + buf.len());
                res.push(ResultCode::Return as u8);
                res.extend_from_slice(buf);
                res
            }
            RunOk::UserError(val) => {
                let mut res = vec![ResultCode::UserError as u8];
                match val {
                    calldata::Value::Str(s) => {
                        res.extend_from_slice(s.as_bytes());
                    }
                    other => {
                        res.extend_from_slice(&0u32.to_le_bytes());
                        res.extend_from_slice(&calldata::encode(other));
                    }
                }
                res
            }
            RunOk::VMError(buf, _) => {
                let mut res = Vec::with_capacity(1 + buf.0.len());
                res.push(ResultCode::VmError as u8);
                res.extend_from_slice(buf.0.as_bytes());
                res
            }
        }
    }
}

impl std::fmt::Display for RunOk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Return(r) => {
                let str = util::str::decode_utf8(r.iter().cloned())
                    .map(|r| match r {
                        Ok('\\') => "\\\\".into(),
                        Ok(c) if c.is_control() || c == '\n' || c == '\x07' => {
                            if c as u32 <= 255 {
                                format!("\\x{:02x}", c as u32)
                            } else {
                                format!("\\u{:04x}", c as u32)
                            }
                        }
                        Ok(c) => c.to_string(),
                        Err(util::str::InvalidSequence(seq)) => {
                            seq.iter().map(|c| format!("\\{:02x}", *c as u32)).join("")
                        }
                    })
                    .join("");
                f.write_fmt(format_args!("Return(\"{str}\")"))
            }
            Self::UserError(r) => write!(f, "UserError({:?})", r),
            Self::VMError(r, _) => f.debug_tuple("VMError").field(r).finish(),
        }
    }
}
pub struct WasmtimeStoreData {
    pub(super) genlayer_ctx: wasi::Context,
    pub(super) limits: rt::memlimiter::Limiter,
    pub(super) supervisor: std::sync::Arc<rt::supervisor::Supervisor>,
}

impl WasmtimeStoreData {
    pub fn genlayer_ctx_mut(&mut self) -> &mut wasi::Context {
        &mut self.genlayer_ctx
    }
}

pub struct VM<T> {
    pub(super) vm_base: VMBase,
    pub(super) data: T,
}

impl VM<wasmtime::Instance> {
    pub async fn run(
        mut self,
    ) -> Result<RunResult, (anyhow::Error, wasi::genlayer_sdk::SingleVMData)> {
        log_debug!(
            wasi_preview1: serde = self.vm_base.store.data().genlayer_ctx.preview1.log(),
            genlayer_sdk: serde = self.vm_base.store.data().genlayer_ctx.genlayer_sdk.log();
            "run"
        );

        let func = self
            .data
            .get_typed_func::<(), ()>(&mut self.vm_base.store, "")
            .or_else(|_| {
                self.data
                    .get_typed_func::<(), ()>(&mut self.vm_base.store, "_start")
            });

        let func = match func {
            Ok(func) => func,
            Err(e) => {
                return Ok(RunResult {
                    run_ok: RunOk::VMError(
                        public_abi::VmError::invalid_contract().wasm().entrypoint(),
                        Some(crate::wasmtime_to_anyhow(e)),
                    ),
                    fingerprint: None,
                    vm_data: self
                        .vm_base
                        .store
                        .into_data()
                        .genlayer_ctx
                        .genlayer_sdk
                        .data,
                });
            }
        };

        log_debug!("execution start");
        let time_start = std::time::Instant::now();
        let res = func.call_async(&mut self.vm_base.store, ()).await;
        log_debug!(
            elapsed:? = self.vm_base.store.data().genlayer_ctx.genlayer_sdk.start_time.elapsed(),
            wasm_start_elapsed:? = time_start.elapsed();
            "vm execution finished"
        );
        let res: anyhow::Result<(rt::vm::RunOk, Option<rt::errors::Fingerprint>)> = match res {
            Ok(()) => Ok((rt::vm::RunOk::empty_return(), None)),
            Err(e) => {
                let e = rt::errors::UnwrapDynError::from(e);
                if self.vm_base.config_copy.needs_error_fingerprint {
                    rt::errors::unwrap_vm_errors_fingerprint(e).map(|(a, b)| (a, Some(b)))
                } else {
                    rt::errors::unwrap_vm_errors(e).map(|a| (a, None))
                }
            }
        };
        let res = if self
            .vm_base
            .store
            .data()
            .supervisor
            .shared_data
            .cancellation
            .is_cancelled()
        {
            match res {
                Ok((rt::vm::RunOk::VMError(msg, cause), fp)) => Ok((
                    rt::vm::RunOk::VMError(
                        public_abi::VmError::timeout(),
                        cause.map(|v| v.context(msg.0)),
                    ),
                    fp,
                )),
                Ok(r) => Ok(r),
                Err(e) => Ok((
                    rt::vm::RunOk::VMError(public_abi::VmError::timeout(), Some(e)),
                    None,
                )),
            }
        } else {
            res
        };
        match &res {
            Ok((rt::vm::RunOk::Return(_), _)) => {
                log_debug!(result = "Return"; "execution result unwrapped")
            }
            Ok((rt::vm::RunOk::UserError(msg), _)) => {
                log_debug!(result = "UserError", message:serde = msg; "execution result unwrapped")
            }
            Ok((rt::vm::RunOk::VMError(e, cause), _)) => {
                log_debug!(result = "VMError", message = e.0, cause:? = cause; "execution result unwrapped")
            }
            Err(e) => {
                log_debug!(result = "Error", error:ah = e; "execution result unwrapped")
            }
        };

        match res {
            Ok((run_ok, fingerprint)) => Ok(RunResult {
                run_ok,
                fingerprint,
                vm_data: self
                    .vm_base
                    .store
                    .into_data()
                    .genlayer_ctx
                    .genlayer_sdk
                    .data,
            }),
            Err(e) => Err((
                e,
                self.vm_base
                    .store
                    .into_data()
                    .genlayer_ctx
                    .genlayer_sdk
                    .data,
            )),
        }
    }
}

impl<T> VM<T> {
    pub fn map(mut self, f: impl FnOnce(&mut VMBase, T) -> T) -> VM<T> {
        VM {
            data: f(&mut self.vm_base, self.data),
            vm_base: self.vm_base,
        }
    }
}

pub struct VMBase {
    pub(super) store: wasmtime::Store<WasmtimeStoreData>,
    pub(super) linker: wasmtime::Linker<WasmtimeStoreData>,
    pub(super) config_copy: wasi::base::Config,
}
