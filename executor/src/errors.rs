use std::collections::BTreeMap;

use crate::public_abi;

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

    pub fn unwrap_res(res: crate::vm::RunResult) -> crate::vm::RunResult {
        match res {
            Ok(x) => Ok(x),
            Err(e) => match e.downcast::<VMError>() {
                Ok(ce) => Ok((crate::vm::RunOk::VMError(ce.0, ce.1), None)),
                Err(e) => Err(e),
            },
        }
    }
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
