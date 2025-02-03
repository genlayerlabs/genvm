pub mod errors;
mod host;
pub mod mmap;
pub mod plugin_loader;
pub mod runner;
pub mod string_templater;
pub mod ustar;
pub mod vm;
pub mod wasi;

pub mod caching;

use errors::ContractError;
pub use host::{AccountAddress, GenericAddress, Host, MessageData};

use anyhow::{Context, Result};
use serde::Deserialize;
use vm::RunOk;
use std::{
    collections::HashMap,
    sync::Arc,
};
use ustar::SharedBytes;

#[derive(Deserialize)]
struct ConfigModule {
    config: serde_json::Value,
}

#[derive(Deserialize)]
struct ConfigModules {
    llm: ConfigModule,
    web: ConfigModule,
}

#[derive(Deserialize)]
struct ConfigSchema {
    modules: ConfigModules,
}

//extern "Rust" {
//    fn new_web_module(
//        args: genvm_modules_interfaces::CtorArgs<'_>,
//    ) -> anyhow::Result<Box<dyn genvm_modules_interfaces::Web + Send + Sync>>;
//
//    fn new_llm_module(
//        args: genvm_modules_interfaces::CtorArgs<'_>,
//    ) -> anyhow::Result<Box<dyn genvm_modules_interfaces::Llm + Send + Sync>>;
//}

fn create_modules(config_path: &String, cancellation: Arc<genvm_modules_interfaces::CancellationToken>) -> Result<vm::Modules> {
    let mut root_path = std::env::current_exe().with_context(|| "getting current exe")?;
    root_path.pop();
    root_path.pop();
    let root_path = root_path
        .into_os_string()
        .into_string()
        .map_err(|_e| anyhow::anyhow!("can't convert path to string"))?;

    let vars: HashMap<String, String> = HashMap::from([("genvmRoot".into(), root_path)]);

    let config_path = string_templater::patch_str(&vars, &config_path)?;
    let config_str = std::fs::read_to_string(std::path::Path::new(&config_path))?;
    let config: serde_json::Value = serde_json::from_str(&config_str)?;
    let config = string_templater::patch_value(&vars, config)?;
    let config: ConfigSchema = serde_json::from_value(config)?;

    let llm_config = serde_json::to_string(&config.modules.llm.config)?;
    let llm = genvm_modules_default_llm::new_llm_module(genvm_modules_interfaces::CtorArgs {
        config: &llm_config,
        cancellation: cancellation.clone(),
    })
    .with_context(|| "creating llm module")?;

    let web_config = serde_json::to_string(&config.modules.web.config)?;
    let web = genvm_modules_default_web::new_web_module(genvm_modules_interfaces::CtorArgs {
        config: &web_config,
        cancellation,
    })
    .with_context(|| "creating llm module")?;

    Ok(vm::Modules {
        llm: Arc::from(llm),
        web: Arc::from(web),
    })
}

pub fn create_supervisor(
    config_path: &String,
    mut host: Host,
    is_sync: bool,
    cancellation: Arc<genvm_modules_interfaces::CancellationToken>,
) -> Result<Arc<tokio::sync::Mutex<vm::Supervisor>>> {
    let modules = match create_modules(config_path, cancellation.clone()) {
        Ok(modules) => modules,
        Err(e) => {
            let err = Err(e);
            host.consume_result(&err)?;
            return Err(err.unwrap_err());
        }
    };
    let shared_data = Arc::new(crate::vm::SharedData::new(modules, is_sync, cancellation));

    Ok(Arc::new(tokio::sync::Mutex::new(vm::Supervisor::new(
        host,
        shared_data,
    )?)))
}

pub async fn run_with_impl(
    entry_message: MessageData,
    supervisor: Arc<tokio::sync::Mutex<vm::Supervisor>>,
    permissions: &str,
) -> vm::RunResult {
    let (mut vm, instance) = {
        let supervisor_clone = supervisor.clone();
        let mut entrypoint = b"call!".to_vec();

        let mut supervisor = supervisor.lock().await;
        supervisor.host.append_calldata(&mut entrypoint)?;

        let essential_data = wasi::genlayer_sdk::SingleVMData {
            conf: wasi::base::Config {
                is_deterministic: true,
                can_read_storage: permissions.contains("r"),
                can_write_storage: permissions.contains("w"),
                can_send_messages: permissions.contains("s"),
                can_call_others: permissions.contains("c"),
                can_spawn_nondet: true,
                state_mode: crate::host::StorageType::Default,
            },
            message_data: entry_message,
            entrypoint: SharedBytes::new(entrypoint),
            supervisor: supervisor_clone,
        };

        let mut vm = supervisor.spawn(essential_data).await?;
        let instance = supervisor
            .apply_contract_actions(&mut vm)
            .await
            .with_context(|| "getting runner actions")
            .map_err(|cause| crate::errors::ContractError::wrap("runner_actions".into(), cause))?;
        (vm, instance)
    };

    vm.run(&instance).await
}

pub async fn run_with(
    entry_message: MessageData,
    supervisor: Arc<tokio::sync::Mutex<vm::Supervisor>>,
    permissions: &str,
) -> vm::RunResult {
    let res = run_with_impl(entry_message, supervisor.clone(), permissions).await;

    let mut supervisor = supervisor.lock().await;

    let res = if supervisor.shared_data.cancellation.is_cancelled() {
        match res {
            Ok(RunOk::ContractError(msg, cause)) => Ok(RunOk::ContractError("timeout".into(), cause.map(|v| v.context(msg)))),
            Ok(r) => Ok(r),
            Err(e) => Ok(RunOk::ContractError("timeout".into(), Some(e))),
        }
    } else {
        ContractError::unwrap_res(res)
    };

    supervisor.host.consume_result(&res)?;

    res
}
