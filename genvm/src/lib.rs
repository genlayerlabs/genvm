mod driver;

mod host;
pub mod plugin_loader;
pub mod runner;
pub mod string_templater;
pub mod vm;
pub mod wasi;

pub use host::{Address, Host, MessageData};

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

pub fn create_supervisor(
    config_path: &String,
    host: Host,
) -> Result<Arc<Mutex<vm::Supervisor>>> {
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
        let name = match &c.name {
            Some(v) => v,
            None => match c.id {
                ConfigModuleName::llm => "llm",
                ConfigModuleName::web => "web",
            },
        };
        match c.id {
            ConfigModuleName::llm => {
                llm = Some(llm_functions_api::Methods::load_from_lib(
                    path, name, config_str,
                )?);
            }
            ConfigModuleName::web => {
                web = Some(web_functions_api::Methods::load_from_lib(
                    path, name, config_str,
                )?);
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
) -> Result<crate::vm::RunResult> {
    let (mut vm, instance) = {
        let supervisor_clone = supervisor.clone();
        let Ok(mut supervisor) = supervisor.lock() else {
            return Err(anyhow::anyhow!("can't lock supervisor"));
        };
        let mut entrypoint = b"call!".to_vec();
        supervisor.host.append_calldata(&mut entrypoint)?;
        let init_actions = supervisor.get_actions_for(&entry_message.contract_account)?;

        let essential_data = wasi::genlayer_sdk::EssentialGenlayerSdkData {
            conf: wasi::base::Config {
                is_deterministic: true,
                can_read_storage: true,
                can_write_storage: true,
                can_spawn_nondet: true,
            },
            message_data: entry_message,
            entrypoint,
            supervisor: supervisor_clone,
            init_actions,
        };

        let mut vm = supervisor.spawn(essential_data)?;
        let instance = supervisor.apply_actions(&mut vm)?;
        (vm, instance)
    };

    let init_fuel = vm.store.get_fuel().unwrap_or(0);
    let res = vm.run(&instance)?;
    let remaining_fuel = vm.store.get_fuel().unwrap_or(0);
    eprintln!(
        "remaining fuel: {remaining_fuel}\nconsumed fuel:  {}",
        u64::wrapping_sub(init_fuel, remaining_fuel)
    );

    {
        let Ok(mut supervisor) = supervisor.lock() else {
            return Err(anyhow::anyhow!("can't lock supervisor"));
        };
        supervisor.host.consume_result(&res)?;
    }

    Ok(res)
}
