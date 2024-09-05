mod driver;

pub mod vm;
pub mod wasi;
pub mod node_iface;
pub mod plugin_loader;

use anyhow::Result;
use std::sync::{Arc, Mutex};

pub trait RequiredApis: node_iface::InitApi + node_iface::RunnerApi + Send + Sync + genvm_modules_common::interfaces::nondet_functions_api::Trait {}

pub fn run_with_api(mut api: Box<dyn RequiredApis>) -> Result<crate::vm::VMRunResult> {

    let mut entrypoint = b"call!".to_vec();
    let calldata = api.get_calldata()?;
    entrypoint.extend_from_slice(&calldata);

    let init_data = api.get_initial_data()?;

    let supervisor = Arc::new(Mutex::new(vm::Supervisor::new(api)?));

    let (mut vm, instance) = {
        let supervisor_clone = supervisor.clone();
        let Ok(mut supervisor) = supervisor.lock() else { return Err(anyhow::anyhow!("can't lock supervisor")); };
        let init_actions = supervisor.get_actions_for(&init_data.contract_account)?;

        let essential_data = wasi::genlayer_sdk::EssentialGenlayerSdkData {
            conf: wasi::base::Config {
                is_deterministic: true,
                can_read_storage: true,
                can_write_storage: true,
                can_spawn_nondet: true,
            },
            message_data: init_data,
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
    eprintln!("remaining fuel: {remaining_fuel}\nconsumed fuel: {}", u64::wrapping_sub(init_fuel, remaining_fuel));

    Ok(res)
}
