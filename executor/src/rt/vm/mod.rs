use crate::{rt, wasi};

use anyhow::Context;
use genvm_common::*;
use itertools::Itertools;

#[derive(serde::Serialize)]
pub enum RunOk {
    Return(Vec<u8>),
    UserError(String),
    VMError(String, #[serde(skip_serializing)] Option<anyhow::Error>),
}

pub type FullRunOk = (RunOk, Option<rt::errors::Fingerprint>);

impl RunOk {
    pub fn empty_return() -> Self {
        Self::Return([0].into())
    }

    pub fn as_bytes_iter(&self) -> impl Iterator<Item = u8> + '_ {
        use crate::public_abi::ResultCode;
        match self {
            RunOk::Return(buf) => [ResultCode::Return as u8]
                .into_iter()
                .chain(buf.iter().cloned()),
            RunOk::UserError(buf) => [ResultCode::UserError as u8]
                .into_iter()
                .chain(buf.as_bytes().iter().cloned()),
            RunOk::VMError(buf, _) => [ResultCode::VmError as u8]
                .into_iter()
                .chain(buf.as_bytes().iter().cloned()),
        }
    }
}

impl std::fmt::Debug for RunOk {
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
            Self::UserError(r) => f.debug_tuple("UserError").field(r).finish(),
            Self::VMError(r, _) => f.debug_tuple("VMError").field(r).finish(),
        }
    }
}
#[derive(Clone)]
pub struct WasmtimeStoreData {
    pub(super) genlayer_ctx: std::sync::Arc<std::sync::Mutex<wasi::Context>>,
    pub(super) limits: rt::memlimiter::Limiter,
}

impl WasmtimeStoreData {
    pub fn genlayer_ctx_mut(&mut self) -> &mut wasi::Context {
        std::sync::Arc::get_mut(&mut self.genlayer_ctx)
            .expect("wasmtime_wasi is not compatible with threads")
            .get_mut()
            .unwrap()
    }
}

pub struct VM<T> {
    pub(super) vm_base: VMBase,
    pub(super) data: T,
}

impl VM<wasmtime::Instance> {
    pub async fn run(mut self) -> anyhow::Result<rt::vm::FullRunOk> {
        if let Ok(lck) = self.vm_base.store.data().genlayer_ctx.lock() {
            log_debug!(wasi_preview1: serde = lck.preview1.log(), genlayer_sdk: serde = lck.genlayer_sdk.log(); "run");
        }

        let func = self
            .data
            .get_typed_func::<(), ()>(&mut self.vm_base.store, "")
            .or_else(|_| {
                self.data
                    .get_typed_func::<(), ()>(&mut self.vm_base.store, "_start")
            })
            .context("can't find entrypoint")?;
        log_debug!("execution start");
        let time_start = std::time::Instant::now();
        let res = func.call_async(&mut self.vm_base.store, ()).await;
        if let Ok(lck) = self.vm_base.store.data().genlayer_ctx.lock() {
            log_debug!(
                elapsed:? = lck.genlayer_sdk.start_time.elapsed(),
                wasm_start_elapsed:? = time_start.elapsed();
                "vm execution finished"
            );
        }
        let res: anyhow::Result<rt::vm::FullRunOk> = match res {
            Ok(()) => Ok((rt::vm::RunOk::empty_return(), None)),
            Err(e) => {
                if self.vm_base.config_copy.needs_error_fingerprint {
                    rt::errors::unwrap_vm_errors_fingerprint(e).map(|(a, b)| (a, Some(b)))
                } else {
                    rt::errors::unwrap_vm_errors(e).map(|a| (a, None))
                }
            }
        };
        match &res {
            Ok((rt::vm::RunOk::Return(_), _)) => {
                log_debug!(result = "Return"; "execution result unwrapped")
            }
            Ok((rt::vm::RunOk::UserError(msg), _)) => {
                log_debug!(result = "UserError", message = msg; "execution result unwrapped")
            }
            Ok((rt::vm::RunOk::VMError(e, cause), _)) => {
                log_debug!(result = "VMError", message = e, cause:? = cause; "execution result unwrapped")
            }
            Err(e) => {
                log_debug!(result = "Error", error:ah = e; "execution result unwrapped")
            }
        };
        res
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
