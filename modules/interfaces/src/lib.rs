use std::sync::{atomic::AtomicU32, Arc};

use serde_derive::{Deserialize, Serialize};

pub trait Web {
    fn get_webpage(
        &self,
        config: String,
        url: String,
    ) -> tokio::task::JoinHandle<anyhow::Result<Box<[u8]>>>;
}

#[derive(Clone, Deserialize, Serialize)]
pub enum Result<T> {
    Ok(T),
    UserError(serde_json::Value),
    FatalError(serde_json::Value),
}

pub struct ParsedDuration(pub tokio::time::Duration);

struct ParsedDurationVisitor;

impl serde::de::Visitor<'_> for ParsedDurationVisitor {
    type Value = ParsedDuration;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("expected string | null")
    }

    fn visit_none<E>(self) -> std::result::Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(ParsedDuration(tokio::time::Duration::ZERO))
    }

    fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let re = regex::Regex::new(r#"^(\d+)(m?s)$"#).unwrap();
        let caps = re
            .captures(value)
            .ok_or(E::custom("invalid duration format"))?;

        let int_str = caps.get(1).unwrap().as_str();

        let int = u64::from_str_radix(int_str, 10).map_err(E::custom)?;

        match caps.get(2).unwrap().as_str() {
            "s" => Ok(ParsedDuration(tokio::time::Duration::from_secs(int))),
            "ms" => Ok(ParsedDuration(tokio::time::Duration::from_millis(int))),
            _ => Err(E::custom("invalid duration suffix")),
        }
    }
}

impl<'de> serde::Deserialize<'de> for ParsedDuration {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(ParsedDurationVisitor)
    }
}

impl serde::Serialize for ParsedDuration {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let as_str = format!("{}ms", self.0.as_millis());

        serializer.serialize_str(&as_str)
    }
}

pub mod llm {
    use serde_derive::{Deserialize, Serialize};

    #[derive(Clone, Deserialize, Serialize, Copy, PartialEq, Eq)]
    #[serde(rename_all = "kebab-case")]
    pub enum OutputMode {
        Text,
        Json,
    }

    #[derive(Deserialize, Serialize)]
    pub struct ExecPromptConfig {
        response_format: Option<OutputMode>,
    }

    #[derive(Serialize, Deserialize)]
    pub struct PromptIDVarsComparative {
        leader_answer: String,
        validator_answer: String,
        principle: String,
    }

    #[derive(Serialize, Deserialize)]
    pub struct PromptIDVarsNonComparativeValidator {
        task: String,
        criteria: String,
        input: String,
        output: String,
    }

    #[derive(Serialize, Deserialize)]
    pub struct PromptIDVarsNonComparativeLeader {
        task: String,
        criteria: String,
        input: String,
    }

    #[derive(Serialize, Deserialize, Clone, PartialEq, Eq)]
    pub enum PromptPart {
        Text(String),
    }

    #[derive(Serialize, Deserialize)]
    pub enum Message {
        Prompt {
            config: ExecPromptConfig,
            parts: Vec<PromptPart>,
        },

        PromptEqComparative {
            vars: PromptIDVarsComparative,
        },
        PromptEqNonComparativeValidator {
            vars: PromptIDVarsNonComparativeValidator,
        },
        PromptEqNonComparativeLeader {
            vars: PromptIDVarsNonComparativeLeader,
        },
    }
}

pub mod web {
    use serde_derive::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    pub enum RenderMode {
        Text,
        HTML,
    }

    fn no_wait() -> super::ParsedDuration {
        super::ParsedDuration(tokio::time::Duration::ZERO)
    }

    #[derive(Serialize, Deserialize)]
    pub enum Message {
        Render {
            mode: RenderMode,
            url: String,
            #[serde(default = "no_wait")]
            wait_after_loaded: super::ParsedDuration,
        },
    }
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
