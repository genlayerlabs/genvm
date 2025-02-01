pub trait Web {
    fn get_webpage(
        &self,
        config: String,
        url: String,
    ) -> tokio::task::JoinHandle<anyhow::Result<Box<[u8]>>>;
}

pub trait Llm {
    fn exec_prompt(
        &self,
        config: String,
        prompt: String,
    ) -> tokio::task::JoinHandle<anyhow::Result<Box<[u8]>>>;

    fn exec_prompt_id(
        &self,
        id: u8,
        vars: String,
    ) -> tokio::task::JoinHandle<anyhow::Result<Box<[u8]>>>;

    fn eq_principle_prompt<'a>(
        &'a self,
        id: u8,
        vars: &'a str,
    ) -> core::pin::Pin<Box<dyn ::core::future::Future<Output = ModuleResult<bool>> + Send + 'a>>;
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

pub type ModuleResult<T> = std::result::Result<T, ModuleError>;

pub async fn module_result_to_future(
    res: impl std::future::Future<Output = ModuleResult<impl AsRef<[u8]> + Send + Sync>> + Send + Sync,
) -> anyhow::Result<Box<[u8]>> {
    let res = res.await;
    match res {
        Ok(original) => {
            let original = original.as_ref();
            let result = Box::new_uninit_slice(original.len() + 1);
            let mut result = unsafe { result.assume_init() };
            result[0] = 0;
            result[1..].copy_from_slice(original);
            Ok(result)
        }
        Err(ModuleError::Recoverable(rec)) => {
            let original = rec.as_bytes();
            let result = Box::new_uninit_slice(original.len() + 1);
            let mut result = unsafe { result.assume_init() };
            result[0] = 1;
            result[1..].copy_from_slice(original);
            Ok(result)
        }
        Err(ModuleError::Fatal(e)) => Err(e),
    }
}
