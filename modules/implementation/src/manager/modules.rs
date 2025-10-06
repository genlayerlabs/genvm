use genvm_common::*;
use std::{collections::HashMap, sync::Arc};

struct ModuleCanceller(
    pub Box<dyn Fn() + Send + Sync>,
    pub Arc<cancellation::Token>,
);

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, Copy)]
pub enum Type {
    Llm,
    Web,
}

pub struct Ctx {
    cancel: Arc<cancellation::Token>,
    llm_module: tokio::sync::RwLock<Option<ModuleCanceller>>,
    web_module: tokio::sync::RwLock<Option<ModuleCanceller>>,
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

    pub async fn start(&self, req: StartRequest) -> anyhow::Result<()> {
        let mut module_lock = match req.module_type {
            Type::Llm => self.llm_module.write().await,
            Type::Web => self.web_module.write().await,
        };

        if module_lock.is_some() {
            anyhow::bail!("module_already_running");
        }

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
                &mut config_vars,
                base_path,
                &genvm_common::templater::DOLLAR_UNFOLDER_RE,
            )?;
            serde_yaml::from_reader(std::fs::File::open(base_path)?)?
        } else {
            req.config.clone()
        };

        let config = genvm_common::templater::patch_json(
            &mut config_vars,
            config,
            &genvm_common::templater::DOLLAR_UNFOLDER_RE,
        )?;

        let module_task = match req.module_type {
            Type::Llm => {
                let allow_empty_backends = req.allow_empty_backends;
                let config = serde_json::from_value(config)?;
                tokio::task::spawn(crate::llm::run_llm_module(
                    nested_cancel,
                    config,
                    allow_empty_backends,
                ))
            }
            Type::Web => {
                let config = serde_json::from_value(config)?;
                tokio::task::spawn(crate::web::run_web_module(nested_cancel, config))
            }
        };

        tokio::time::sleep(std::time::Duration::from_secs(5)).await;

        if module_task.is_finished() {
            anyhow::bail!("module_failed_to_start");
        }

        // Store the cancellation token
        *module_lock = Some(ModuleCanceller(Box::new(canceller), module_cancel));

        Ok(())
    }

    pub async fn stop(&self, module_type: Type) -> anyhow::Result<bool> {
        let mut module_lock = match module_type {
            Type::Llm => self.llm_module.write().await,
            Type::Web => self.web_module.write().await,
        };

        let Some(cancel_token) = module_lock.take() else {
            return Ok(false);
        };

        cancel_token.0();

        Ok(true)
    }

    pub async fn get_status(&self, module_type: Type) -> &'static str {
        let module_lock = match module_type {
            Type::Llm => self.llm_module.read().await,
            Type::Web => self.web_module.read().await,
        };

        match &*module_lock {
            None => "stopped",
            Some(canceller) => {
                if canceller.1.is_cancelled() {
                    "stopping"
                } else {
                    "running"
                }
            }
        }
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
