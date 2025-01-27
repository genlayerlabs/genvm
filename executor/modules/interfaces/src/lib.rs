pub trait Web {
    fn get_webpage(&self, gas: &mut u64, config: &str, url: &str) -> ModuleResult<String>;
}

pub trait Llm {
    fn exec_prompt(&self, gas: &mut u64, config: &str, prompt: &str) -> ModuleResult<String>;
    fn exec_prompt_id(&self, gas: &mut u64, id: u8, vars: &str) -> ModuleResult<String>;
    fn eq_principle_prompt(&self, gas: &mut u64, id: u8, vars: &str) -> ModuleResult<bool>;
}

#[repr(C)]
pub struct CtorArgs<'a> {
    pub config: &'a str,
}

#[derive(Debug)]
pub enum ModuleError {
    Recoverable(&'static str),
    Fatal(anyhow::Error),
}

impl<T> From<T> for ModuleError
where
    T: Into<anyhow::Error>,
{
    fn from(value: T) -> Self {
        ModuleError::Fatal(value.into())
    }
}

//impl std::fmt::Display for ModuleError {
//    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//        f.write_fmt(format_args!("{:?}", self))
//    }
//}

//impl std::error::Error for ModuleError {}

pub type ModuleResult<T> = std::result::Result<T, ModuleError>;
