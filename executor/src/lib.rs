#![feature(once_wait)]

mod host;
pub mod plugin_loader;
pub mod runner;
pub mod string_templater;
pub mod vm;
pub mod wasi;

pub mod caching;

pub use host::{AccountAddress, GenericAddress, Host, MessageData};

use anyhow::Result;
use genvm_modules_common::interfaces::{llm_functions_api, web_functions_api};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
#[allow(non_camel_case_types)]
enum ConfigModuleName {
    llm,
    web,
}

#[derive(Deserialize)]
struct ConfigModule {
    path: String,
    name: Option<String>,
    id: ConfigModuleName,
    config: serde_json::Value,
}

#[derive(Deserialize)]
struct ConfigSchema {
    modules: Vec<ConfigModule>,
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

pub fn create_supervisor(config_path: &String, host: Host) -> Result<Arc<Mutex<vm::Supervisor>>> {
    use plugin_loader::llm_functions_api::Loader as _;
    use plugin_loader::web_functions_api::Loader as _;

    let mut root_path = std::env::current_exe()?;
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

    let mut llm = None;
    let mut web = None;
    for c in &config.modules {
        let path = std::path::Path::new(&c.path);
        let config_str = serde_json::to_string(&c.config)?;
        let args = genvm_modules_common::CtorArgs {
            version: genvm_modules_common::Version { major: 0, minor: 0 },
            module_config: config_str.as_ptr(),
            module_config_len: config_str.len(),
            thread_pool: fake_thread_pool(),
        };
        let name = match &c.name {
            Some(v) => v,
            None => match c.id {
                ConfigModuleName::llm => "llm",
                ConfigModuleName::web => "web",
            },
        };
        match c.id {
            ConfigModuleName::llm => {
                llm = Some(llm_functions_api::Methods::load_from_lib(path, name, args)?);
            }
            ConfigModuleName::web => {
                web = Some(web_functions_api::Methods::load_from_lib(path, name, args)?);
            }
        }
    }

    let modules = match (llm, web) {
        (Some(llm), Some(web)) => vm::Modules { llm, web },
        _ => anyhow::bail!("some of required modules is not supplied"),
    };

    Ok(Arc::new(Mutex::new(vm::Supervisor::new(modules, host)?)))
}

pub fn run_with(
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
        let init_actions = supervisor.get_actions_for(&entry_message.contract_account)?;

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
            init_actions,
        };

        let mut vm = supervisor.spawn(essential_data)?;
        let instance = supervisor.apply_actions(&mut vm)?;
        (vm, instance)
    };

    let res = vm.run(&instance);

    {
        let Ok(mut supervisor) = supervisor.lock() else {
            anyhow::bail!("can't lock supervisor");
        };
        supervisor.host.consume_result(&res)?;
    }

    res
}
