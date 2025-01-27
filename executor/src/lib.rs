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
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use ustar::SharedBytes;

#[derive(Deserialize)]
struct ConfigModule {
    path: String,
    name: Option<String>,
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

extern "Rust" {
    fn new_web_module(
        args: genvm_modules_interfaces::CtorArgs<'_>,
    ) -> anyhow::Result<Box<dyn genvm_modules_interfaces::Web + Send + Sync>>;
}

extern "Rust" {
    fn new_llm_module(
        args: genvm_modules_interfaces::CtorArgs<'_>,
    ) -> anyhow::Result<Box<dyn genvm_modules_interfaces::Llm + Send + Sync>>;
}

fn create_modules(
    config_path: &String,
    log_fd: std::os::fd::RawFd,
    should_quit: *mut u32,
) -> Result<vm::Modules> {
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
    let llm = unsafe {
        new_llm_module(genvm_modules_interfaces::CtorArgs {
            config: &llm_config,
        })
    }
    .with_context(|| "creating llm module")?;

    let web_config = serde_json::to_string(&config.modules.web.config)?;
    let web = unsafe {
        new_web_module(genvm_modules_interfaces::CtorArgs {
            config: &web_config,
        })
    }
    .with_context(|| "creating llm module")?;

    Ok(vm::Modules { llm, web })
}

pub fn create_supervisor(
    config_path: &String,
    mut host: Host,
    log_fd: std::os::fd::RawFd,
    is_sync: bool,
) -> Result<Arc<Mutex<vm::Supervisor>>> {
    let shared_data = Arc::new(crate::vm::SharedData::new(is_sync));
    let should_quit_ptr = shared_data.should_exit.as_ptr();
    let modules = match create_modules(config_path, log_fd, should_quit_ptr) {
        Ok(modules) => modules,
        Err(e) => {
            let err = Err(e);
            host.consume_result(&err)?;
            return Err(err.unwrap_err());
        }
    };

    Ok(Arc::new(Mutex::new(vm::Supervisor::new(
        modules,
        host,
        shared_data,
    )?)))
}

pub fn run_with_impl(
    entry_message: MessageData,
    supervisor: Arc<Mutex<vm::Supervisor>>,
    permissions: &str,
) -> vm::RunResult {
    let (mut vm, instance) = {
        let supervisor_clone = supervisor.clone();
        let Ok(mut supervisor) = supervisor.lock() else {
            return Err(anyhow::anyhow!("can't lock supervisor"));
        };
        let mut entrypoint = b"call!".to_vec();
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

        let mut vm = supervisor.spawn(essential_data)?;
        let instance = supervisor
            .apply_contract_actions(&mut vm)
            .with_context(|| "getting runner actions")
            .map_err(|cause| crate::errors::ContractError::wrap("runner_actions".into(), cause))?;
        (vm, instance)
    };

    vm.run(&instance)
}

pub fn run_with(
    entry_message: MessageData,
    supervisor: Arc<Mutex<vm::Supervisor>>,
    permissions: &str,
) -> vm::RunResult {
    let res = run_with_impl(entry_message, supervisor.clone(), permissions);
    let res = ContractError::unwrap_res(res);

    {
        let Ok(mut supervisor) = supervisor.lock() else {
            anyhow::bail!("can't lock supervisor");
        };
        supervisor.host.consume_result(&res)?;
    }

    res
}
