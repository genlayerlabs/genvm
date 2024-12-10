#![feature(once_wait)]

pub mod errors;
mod host;
pub mod plugin_loader;
pub mod runner;
pub mod string_templater;
pub mod vm;
pub mod wasi;

pub mod caching;

use errors::ContractError;
pub use host::{AccountAddress, GenericAddress, Host, MessageData};

use anyhow::{Context, Result};
use genvm_modules_common::interfaces::{llm_functions_api, web_functions_api};
use serde::Deserialize;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

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

fn fake_thread_pool() -> genvm_modules_common::SharedThreadPoolABI {
    extern "C-unwind" fn exec(
        _zelf: *const (),
        ctx: *const (),
        cb: extern "C-unwind" fn(ctx: *const ()),
    ) {
        cb(ctx);
    }
    genvm_modules_common::SharedThreadPoolABI {
        ctx: std::ptr::null(),
        submit_task: exec,
    }
}

fn load_mod<T>(
    mc: &ConfigModule,
    default_name: &str,
    f: impl FnOnce(&std::path::Path, &str, genvm_modules_common::CtorArgs) -> Result<T>,
    log_fd: std::os::fd::RawFd,
) -> Result<T> {
    let config_str = serde_json::to_string(&mc.config)?;
    let args = genvm_modules_common::CtorArgs {
        version: genvm_modules_common::Version { major: 0, minor: 0 },
        module_config: config_str.as_ptr(),
        module_config_len: config_str.len(),
        thread_pool: fake_thread_pool(),
        log_fd,
    };
    f(
        &std::path::Path::new(&mc.path),
        match &mc.name {
            Some(v) => v,
            None => default_name,
        },
        args,
    )
}

fn create_modules(config_path: &String, log_fd: std::os::fd::RawFd) -> Result<vm::Modules> {
    use plugin_loader::llm_functions_api::Loader as _;
    use plugin_loader::web_functions_api::Loader as _;

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

    let llm = load_mod(
        &config.modules.llm,
        "llm",
        llm_functions_api::Methods::load_from_lib,
        log_fd,
    )?;
    let web = load_mod(
        &config.modules.web,
        "web",
        web_functions_api::Methods::load_from_lib,
        log_fd,
    )?;

    Ok(vm::Modules { llm, web })
}

pub fn create_supervisor(
    config_path: &String,
    mut host: Host,
    log_fd: std::os::fd::RawFd,
    is_sync: bool,
) -> Result<Arc<Mutex<vm::Supervisor>>> {
    let modules = match create_modules(config_path, log_fd) {
        Ok(modules) => modules,
        Err(e) => {
            let err = Err(e);
            host.consume_result(&err)?;
            return Err(err.unwrap_err());
        }
    };

    Ok(Arc::new(Mutex::new(vm::Supervisor::new(
        modules, host, is_sync,
    )?)))
}

pub fn run_with_impl(
    entry_message: MessageData,
    supervisor: Arc<Mutex<vm::Supervisor>>,
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
                can_read_storage: true,
                can_write_storage: true,
                can_spawn_nondet: true,
            },
            message_data: entry_message,
            entrypoint: entrypoint.into(),
            supervisor: supervisor_clone,
        };

        let mut vm = supervisor.spawn(essential_data)?;
        let instance = supervisor.apply_contract_actions(&mut vm)
            .with_context(|| "getting runner actions")
            .map_err(|cause| crate::errors::ContractError::wrap("runner_actions".into(), cause))?;
        (vm, instance)
    };

    vm.run(&instance)
}

pub fn run_with(
    entry_message: MessageData,
    supervisor: Arc<Mutex<vm::Supervisor>>,
) -> vm::RunResult {
    let res = run_with_impl(entry_message, supervisor.clone());
    let res = ContractError::unwrap_res(res);

    {
        let Ok(mut supervisor) = supervisor.lock() else {
            anyhow::bail!("can't lock supervisor");
        };
        supervisor.host.consume_result(&res)?;
    }

    res
}
