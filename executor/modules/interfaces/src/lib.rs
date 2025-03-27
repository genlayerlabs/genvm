use std::sync::{atomic::AtomicU32, Arc};

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
pub struct CtorArgs {
    pub config: serde_yaml::Value,
    pub cancellation: Arc<CancellationToken>,
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

pub struct CancellationToken {
    pub chan: tokio::sync::mpsc::Sender<()>,
    pub should_quit: Arc<AtomicU32>,
}

impl CancellationToken {
    pub fn is_cancelled(&self) -> bool {
        self.should_quit.load(std::sync::atomic::Ordering::SeqCst) != 0
    }
}

pub fn make_cancellation() -> (Arc<CancellationToken>, impl Clone + Fn() -> ()) {
    let (sender, receiver) = tokio::sync::mpsc::channel(1);

    let cancel = Arc::new(CancellationToken {
        chan: sender,
        should_quit: Arc::new(AtomicU32::new(0)),
    });

    let cancel_copy = cancel.clone();
    let receiver = Arc::new(std::sync::Mutex::new(receiver));

    (cancel, move || {
        cancel_copy
            .should_quit
            .store(1, std::sync::atomic::Ordering::SeqCst);
        if let Ok(mut receiver) = receiver.lock() {
            receiver.close();
        }
    })
}
