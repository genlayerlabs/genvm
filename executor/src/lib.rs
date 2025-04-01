pub mod caching;
pub mod errors;
mod host;
pub mod mmap;
pub mod modules;
pub mod runner;
pub mod ustar;
pub mod vm;
pub mod wasi;

use errors::ContractError;
use host::AbsentLeaderResult;
pub use host::{AccountAddress, GenericAddress, Host, MessageData};

use anyhow::{Context, Result};
use serde::Deserialize;
use std::{collections::HashMap, sync::Arc};
use ustar::SharedBytes;
use vm::{Modules, RunOk};

#[derive(Deserialize)]
struct ConfigModule {
    address: String,
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

pub fn create_supervisor(
    config_path: &str,
    host: Host,
    is_sync: bool,
    cancellation: Arc<genvm_common::cancellation::Token>,
) -> Result<Arc<tokio::sync::Mutex<vm::Supervisor>>> {
    let mut root_path = std::env::current_exe().with_context(|| "getting current exe")?;
    root_path.pop();
    root_path.pop();
    let root_path = root_path
        .into_os_string()
        .into_string()
        .map_err(|e| anyhow::anyhow!("can't convert path to string `{e:?}`"))?;

    let vars: HashMap<String, String> = HashMap::from([("genvmRoot".into(), root_path)]);

    let config = genvm_common::load_config(&vars, config_path).with_context(|| "loading config")?;
    let config: ConfigSchema = serde_yaml::from_value(config)?;

    let modules = Modules {
        web: Arc::new(modules::Module::new(config.modules.web.address)),
        llm: Arc::new(modules::Module::new(config.modules.llm.address)),
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
        supervisor.host.get_calldata(&mut entrypoint)?;

        let essential_data = wasi::genlayer_sdk::SingleVMData {
            conf: wasi::base::Config {
                is_deterministic: true,
                can_read_storage: permissions.contains("r"),
                can_write_storage: permissions.contains("w"),
                can_send_messages: permissions.contains("s"),
                can_call_others: permissions.contains("c"),
                can_spawn_nondet: permissions.contains("n"),
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
            Ok(RunOk::ContractError(msg, cause)) => Ok(RunOk::ContractError(
                "timeout".into(),
                cause.map(|v| v.context(msg)),
            )),
            Ok(r) => Ok(r),
            Err(e) => Ok(RunOk::ContractError("timeout".into(), Some(e))),
        }
    } else {
        ContractError::unwrap_res(res)
    };

    let res = match res {
        Err(e) => match e.downcast() {
            Ok(AbsentLeaderResult) => {
                Ok(RunOk::ContractError("deterministic_violation".into(), None))
            }
            Err(e) => {
                log::error!(error:? = e; "internal error");
                Err(e)
            }
        },
        e => e,
    };

    supervisor.host.consume_result(&res)?;

    res
}
