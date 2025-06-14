#[derive(Debug)]
pub struct ContractError(pub String, pub Option<anyhow::Error>);

impl std::error::Error for ContractError {}

impl std::fmt::Display for ContractError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "VMError({})", self.0)
    }
}

impl ContractError {
    pub fn oom(cause: Option<anyhow::Error>) -> Self {
        ContractError("OOM".to_owned(), cause)
    }

    pub fn wrap(message: String, cause: anyhow::Error) -> Self {
        match cause.downcast::<ContractError>() {
            Err(cause) => Self(message, Some(cause)),
            Ok(v) => v,
        }
    }

    pub fn unwrap_res(res: crate::vm::RunResult) -> crate::vm::RunResult {
        match res {
            Ok(x) => Ok(x),
            Err(e) => match e.downcast::<ContractError>() {
                Ok(ce) => Ok(crate::vm::RunOk::VMError(ce.0, ce.1)),
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
