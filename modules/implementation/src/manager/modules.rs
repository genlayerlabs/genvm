use genvm_common::io::Stream;
use genvm_common::*;
use std::{collections::BTreeMap, collections::HashMap, future::Future, pin::Pin, sync::Arc};

use super::execution_context::{ExecutionContext, LlmSubContext, WebSubContext};

/// Type-erased handler that accepts a client stream and processes it
pub type StreamHandler = Arc<
    dyn Fn(
            Box<dyn Stream>,
            Option<sync::DArc<ExecutionContext>>,
        ) -> Pin<Box<dyn Future<Output = ()> + Send>>
        + Send
        + Sync,
>;

struct ModuleStateBase {
    canceller: Box<dyn Fn() + Send + Sync>,
    cancel_token: Arc<cancellation::Token>,
    handler: StreamHandler,
    _task: tokio::task::JoinHandle<anyhow::Result<()>>,
}

pub(crate) struct ModuleStateLlm {
    base: ModuleStateBase,
    config: sync::DArc<crate::llm::config::Config>,
    providers: Arc<BTreeMap<String, Box<dyn crate::llm::providers::Provider + Send + Sync>>>,
}

pub(crate) struct ModuleStateWeb {
    base: ModuleStateBase,
    config: sync::DArc<crate::web::config::Config>,
}

impl ModuleStateLlm {
    pub fn create_sub_context(
        &self,
        hello: &Arc<genvm_modules_interfaces::GenVMHello>,
    ) -> anyhow::Result<LlmSubContext> {
        let metrics = sync::DArc::new(crate::llm::Metrics::default());
        let scripting = crate::scripting::create_ctx_part(
            hello,
            &self.config.gep(|x| &x.mod_base),
            metrics.gep(|x| &x.scripting),
        )?;
        let module = crate::llm::ctx::CtxPart {
            providers: self.providers.clone(),
            metrics,
        };
        Ok(LlmSubContext { scripting, module })
    }
}

impl ModuleStateWeb {
    pub fn create_sub_context(
        &self,
        hello: &Arc<genvm_modules_interfaces::GenVMHello>,
    ) -> anyhow::Result<WebSubContext> {
        let metrics = sync::DArc::new(crate::web::Metrics::default());
        let scripting = crate::scripting::create_ctx_part(
            hello,
            &self.config.gep(|x| &x.mod_base),
            metrics.gep(|x| &x.scripting),
        )?;
        Ok(WebSubContext { scripting })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, Copy)]
pub enum Type {
    Llm,
    Web,
}

pub struct Ctx {
    cancel: Arc<cancellation::Token>,
    llm_module: tokio::sync::RwLock<Option<ModuleStateLlm>>,
    web_module: tokio::sync::RwLock<Option<ModuleStateWeb>>,
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct StartRequest {
    pub module_type: Type,
    pub config: serde_json::Value,
    #[serde(default)]
    pub allow_empty_backends: bool,
}

impl Ctx {
    pub fn new(cancel: Arc<cancellation::Token>) -> Self {
        Self {
            cancel,
            llm_module: tokio::sync::RwLock::new(None),
            web_module: tokio::sync::RwLock::new(None),
        }
    }

    pub async fn create_execution_context(
        &self,
        hello: Arc<genvm_modules_interfaces::GenVMHello>,
    ) -> anyhow::Result<sync::DArc<ExecutionContext>> {
        let llm = if let Some(state) = self.llm_module.read().await.as_ref() {
            Some(state.create_sub_context(&hello)?)
        } else {
            None
        };
        let web = if let Some(state) = self.web_module.read().await.as_ref() {
            Some(state.create_sub_context(&hello)?)
        } else {
            None
        };
        Ok(sync::DArc::new(ExecutionContext { hello, llm, web }))
    }

    pub async fn start(&self, req: StartRequest) -> anyhow::Result<()> {
        let (module_cancel, canceller) = genvm_common::cancellation::make();

        // Set up cancellation that triggers when either parent cancels or we explicitly cancel
        let parent_cancel = self.cancel.clone();
        let nested_cancel = module_cancel.clone();

        let canceller_nested = canceller.clone();
        let nested_cancel_2 = nested_cancel.clone();
        tokio::spawn(async move {
            tokio::select! {
                _ = parent_cancel.chan.closed() => {
                    canceller_nested();
                }
                _ = nested_cancel_2.chan.closed() => {
                }
            }
        });

        let mut config_vars = HashMap::new();
        genvm_common::populate_default_config_vars(&mut config_vars)?;

        let config = if req.config.is_null() {
            let base_path = match req.module_type {
                Type::Llm => "${exeDir}/../config/genvm-module-llm.yaml",
                Type::Web => "${exeDir}/../config/genvm-module-web.yaml",
            };
            let base_path = genvm_common::templater::patch_str(
                &config_vars,
                base_path,
                &genvm_common::templater::DOLLAR_UNFOLDER_RE,
            )?;
            serde_yaml::from_reader(std::fs::File::open(base_path)?)?
        } else {
            req.config.clone()
        };

        let config = genvm_common::templater::patch_json(
            &config_vars,
            config,
            &genvm_common::templater::DOLLAR_UNFOLDER_RE,
        )?;

        match req.module_type {
            Type::Llm => {
                let mut module_lock = self.llm_module.write().await;
                if module_lock.is_some() {
                    anyhow::bail!("module_already_running");
                }

                let allow_empty_backends = req.allow_empty_backends;
                let config = serde_json::from_value(config)?;
                let (handler, bind_future, llm_config, providers) =
                    crate::llm::create_llm_module(nested_cancel, config, allow_empty_backends)
                        .await?;
                let bind_future: std::pin::Pin<
                    Box<dyn std::future::Future<Output = anyhow::Result<()>> + Send>,
                > = Box::pin(bind_future);

                let module_task = tokio::task::spawn(bind_future);

                *module_lock = Some(ModuleStateLlm {
                    base: ModuleStateBase {
                        canceller: Box::new(canceller),
                        cancel_token: module_cancel,
                        handler,
                        _task: module_task,
                    },
                    config: llm_config,
                    providers,
                });
            }
            Type::Web => {
                let mut module_lock = self.web_module.write().await;
                if module_lock.is_some() {
                    anyhow::bail!("module_already_running");
                }

                let config = serde_json::from_value(config)?;
                let (handler, bind_future, web_config) =
                    crate::web::create_web_module(nested_cancel, config).await?;
                let bind_future: std::pin::Pin<
                    Box<dyn std::future::Future<Output = anyhow::Result<()>> + Send>,
                > = Box::pin(bind_future);

                let module_task = tokio::task::spawn(bind_future);

                *module_lock = Some(ModuleStateWeb {
                    base: ModuleStateBase {
                        canceller: Box::new(canceller),
                        cancel_token: module_cancel,
                        handler,
                        _task: module_task,
                    },
                    config: web_config,
                });
            }
        }

        Ok(())
    }

    pub async fn stop(&self, module_type: Type) -> anyhow::Result<bool> {
        match module_type {
            Type::Llm => {
                let mut module_lock = self.llm_module.write().await;
                let Some(state) = module_lock.take() else {
                    return Ok(false);
                };
                (state.base.canceller)();
            }
            Type::Web => {
                let mut module_lock = self.web_module.write().await;
                let Some(state) = module_lock.take() else {
                    return Ok(false);
                };
                (state.base.canceller)();
            }
        }
        Ok(true)
    }

    pub async fn get_status(&self, module_type: Type) -> &'static str {
        match module_type {
            Type::Llm => {
                let module_lock = self.llm_module.read().await;
                match &*module_lock {
                    None => "stopped",
                    Some(state) => {
                        if state.base.cancel_token.is_cancelled() {
                            "stopping"
                        } else {
                            "running"
                        }
                    }
                }
            }
            Type::Web => {
                let module_lock = self.web_module.read().await;
                match &*module_lock {
                    None => "stopped",
                    Some(state) => {
                        if state.base.cancel_token.is_cancelled() {
                            "stopping"
                        } else {
                            "running"
                        }
                    }
                }
            }
        }
    }

    /// Get the handler for a running module (returns None if module not running)
    pub async fn get_handler(&self, module_type: Type) -> Option<StreamHandler> {
        match module_type {
            Type::Llm => {
                let module_lock = self.llm_module.read().await;
                module_lock.as_ref().map(|state| state.base.handler.clone())
            }
            Type::Web => {
                let module_lock = self.web_module.read().await;
                module_lock.as_ref().map(|state| state.base.handler.clone())
            }
        }
    }

    /// Get both module handlers if both are running
    pub async fn get_handlers(&self) -> Option<(StreamHandler, StreamHandler)> {
        let llm = self.get_handler(Type::Llm).await?;
        let web = self.get_handler(Type::Web).await?;
        Some((llm, web))
    }

    pub async fn get_module_locks(zelf: sync::DArc<Ctx>) -> Option<impl std::any::Any> {
        let llm_lock = zelf
            .clone()
            .into_get_sub_async(|x| x.llm_module.read())
            .await;
        if llm_lock.is_none() {
            return None;
        }
        let web_lock = zelf
            .clone()
            .into_get_sub_async(|x| x.web_module.read())
            .await;
        if web_lock.is_none() {
            return None;
        }
        Some((llm_lock, web_lock))
    }
}
