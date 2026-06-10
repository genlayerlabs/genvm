use crate::{public_abi, rt, wasi};

use genlayer_calldata::codec::Encode;
use genlayer_sdk::abi;
use genvm_common::*;
use itertools::Itertools;

pub mod storage;

#[derive(Debug)]
pub enum RunOk {
    Return(calldata::unparsed::Maybe<calldata::Value>),
    UserError(calldata::unparsed::Maybe<calldata::Value>),
    VMError(abi::consts::VmError, Option<anyhow::Error>),
}

pub struct RunResult {
    pub run_ok: RunOk,
    pub backtrace: Option<rt::errors::Backtrace>,
    pub wasm_store_hashes: rt::errors::WasmStoreHashes,
    pub vm_data: Box<wasi::genlayer_sdk::SingleVMData>,
}

#[derive(Debug, Clone, genlayer_calldata::Encode)]
pub struct FullResult {
    pub kind: public_abi::ResultCode,
    pub data: calldata::unparsed::Maybe<calldata::Value>,
    pub backtrace: Option<rt::errors::Backtrace>,
    pub wasm_store_hashes: rt::errors::WasmStoreHashes,
    pub subvm_hashes: bytes::Bytes,
    pub storage_changes: Vec<storage::Delta>,

    pub emissions: Vec<domain::ExecutionEmission>,
}

/// Digest of an empty sub-VM hash accumulator, i.e. the value a run with no
/// deterministic sub-calls produces. Kept in sync with how [`RunResult`]'s
/// `det_subvm_hashes` is finalized so error/edge results hash uniformly.
pub(crate) fn empty_subvm_hashes() -> bytes::Bytes {
    bytes::Bytes::from(sha3::Digest::finalize(sha3::Sha3_256::default()).to_vec())
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
                RunOk::Return(buf) => buf,
                RunOk::UserError(val) => val,
                RunOk::VMError(msg, _) => calldata::Value::Str(msg.into()).into(),
            },
            backtrace: None,
            wasm_store_hashes: rt::errors::WasmStoreHashes::default(),
            subvm_hashes: empty_subvm_hashes(),
            storage_changes: Vec::new(),
            emissions: Vec::new(),
        }
    }

    pub fn timeout() -> Self {
        Self {
            kind: public_abi::ResultCode::VmError,
            data: calldata::Value::Str(public_abi::VmError::timeout().into()).into(),
            backtrace: None,
            wasm_store_hashes: rt::errors::WasmStoreHashes::default(),
            subvm_hashes: empty_subvm_hashes(),
            storage_changes: Vec::new(),
            emissions: Vec::new(),
        }
    }
}

impl RunOk {
    pub fn empty_return() -> Self {
        Self::Return(calldata::Value::Null.into())
    }

    pub fn as_bytes(&self) -> Vec<u8> {
        use crate::public_abi::ResultCode;
        match self {
            RunOk::Return(buf) => {
                let encoded = calldata::encode_obj(buf);
                let mut res = Vec::with_capacity(1 + encoded.len());
                res.push(ResultCode::Return as u8);
                res.extend_from_slice(&encoded);
                res
            }
            RunOk::UserError(val) => {
                let mut res = vec![ResultCode::UserError as u8];
                match val {
                    calldata::unparsed::Maybe::Materialized(calldata::Value::Str(s)) => {
                        res.extend_from_slice(s.as_bytes());
                    }
                    calldata::unparsed::Maybe::Materialized(other) => {
                        res.extend_from_slice(&0u32.to_le_bytes());
                        res.extend_from_slice(&calldata::encode(other));
                    }
                    // Already-encoded calldata bytes; emit them in the tagged form.
                    calldata::unparsed::Maybe::Checked(raw) => {
                        res.extend_from_slice(&0u32.to_le_bytes());
                        res.extend_from_slice(&raw.0);
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
                let encoded = calldata::encode_obj(r);
                let str = util::str::decode_utf8(encoded.iter().cloned())
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
    pub async fn run(mut self) -> Result<RunResult, rt::SpawnError> {
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
                    backtrace: None,
                    wasm_store_hashes: self.vm_base.wasm_store_hashes(),
                    vm_data: Box::new(
                        self.vm_base
                            .store
                            .into_data()
                            .genlayer_ctx
                            .genlayer_sdk
                            .data,
                    ),
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
        let res: anyhow::Result<(rt::vm::RunOk, Option<rt::errors::Backtrace>)> = match res {
            Ok(()) => Ok((rt::vm::RunOk::empty_return(), None)),
            Err(e) => {
                let e = rt::errors::UnwrapDynError::from(e);
                if self.vm_base.config_copy.needs_error_fingerprint {
                    // The store is still alive here and holds the wasm memory
                    // state as left by the trapping execution, so take the
                    // memory fingerprint directly from it.
                    rt::errors::unwrap_vm_errors_backtrace(e)
                } else {
                    rt::errors::unwrap_vm_errors(e).map(|run_ok| (run_ok, None))
                }
            }
        };

        let wasm_store_hashes = self.vm_base.wasm_store_hashes();

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
                Ok((rt::vm::RunOk::VMError(msg, cause), backtrace)) => Ok((
                    rt::vm::RunOk::VMError(
                        public_abi::VmError::timeout(),
                        cause.map(|v| v.context(msg.0)),
                    ),
                    backtrace,
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
                log_debug!(result = "UserError", message:cd = msg.clone(); "execution result unwrapped")
            }
            Ok((rt::vm::RunOk::VMError(e, cause), _)) => {
                log_debug!(result = "VMError", message = e.0, cause:? = cause; "execution result unwrapped")
            }
            Err(e) => {
                log_debug!(result = "Error", error:ah = e; "execution result unwrapped")
            }
        };

        match res {
            Ok((run_ok, backtrace)) => {
                let vm_data = Box::new(
                    self.vm_base
                        .store
                        .into_data()
                        .genlayer_ctx
                        .genlayer_sdk
                        .data,
                );
                Ok(RunResult {
                    run_ok,
                    backtrace,
                    wasm_store_hashes,
                    vm_data,
                })
            }
            Err(e) => Err(rt::SpawnError {
                error: e,
                state: Box::new(rt::SpawnErrorState::Spawned(self.vm_base)),
            }),
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

impl VMBase {
    pub fn wasm_store_hashes(&mut self) -> rt::errors::WasmStoreHashes {
        rt::errors::WasmStoreHashes(self.store.fingerprint().module_instances)
    }
}

/// A [`calldata::Writer`] that feeds the encoded bytes straight into a sha3
/// digest, used to hash a value without materializing its encoding.
pub(crate) struct Sha3Writer(pub sha3::Sha3_256);

impl calldata::Writer for &mut Sha3Writer {
    type Error = std::convert::Infallible;

    fn write_all(&mut self, data: &[u8]) -> Result<(), Self::Error> {
        sha3::Digest::update(&mut self.0, data);
        Ok(())
    }
}

impl RunResult {
    fn small_hash_impl(&self) -> std::result::Result<[u8; 32], std::convert::Infallible> {
        use sha3::Digest;

        let mut hasher = Sha3Writer(sha3::Sha3_256::new());

        let mut enc = calldata::Encoder::new(&mut hasher);

        enc.start_map(4)?;
        enc.push_map_k("kind")?;
        match &self.run_ok {
            RunOk::Return(_) => enc.push_str("Return")?,
            RunOk::UserError(_) => enc.push_str("UserError")?,
            RunOk::VMError(_, _) => enc.push_str("VMError")?,
        }
        enc.push_map_k("result")?;
        match &self.run_ok {
            RunOk::Return(buf) => {
                buf.encode(&mut enc)?;
            }
            RunOk::UserError(buf) => {
                buf.encode(&mut enc)?;
            }
            RunOk::VMError(data, _) => {
                enc.push_str(&data.0)?;
            }
        }

        enc.push_map_k("subvm_hashes")?;
        enc.push_bytes(&self.vm_data.det_subvm_hashes.clone().finalize())?;

        enc.push_map_k("wasm_store_hashes")?;
        self.wasm_store_hashes.encode(&mut enc)?;

        Ok(hasher.0.finalize().into())
    }

    pub fn small_hash(&self) -> [u8; 32] {
        match self.small_hash_impl() {
            Ok(hash) => hash,
            Err(e) => match e {},
        }
    }
}
