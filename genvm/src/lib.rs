mod driver;

pub mod vm;
pub mod wasi;
pub mod node_iface;

use anyhow::{Context as _, Result};
use node_iface::{InitAction, MessageData};
use vm::InitActions;
use wasi::{genlayer_sdk, preview1};
use core::str;
use std::{borrow::BorrowMut, path::Path, sync::{Arc, Mutex}};

pub trait RequiredApis: node_iface::InitApi + node_iface::RunnerApi + Send + Sync {}

pub fn get_actions(supervisor: &mut vm::Supervisor, message_data: &MessageData) -> Result<InitActions> {
    let code: Arc<Vec<u8>> = supervisor.api.get_code(&message_data.contract_account)?;
    let actions =
        if wasmparser::Parser::is_core_wasm(&code[..]) {
            Vec::from([InitAction::StartWasm { contents: code.clone(), debug_path: Some("<contract>".into()) }])
        } else {
            let code_str = str::from_utf8(&code[..])?;
            let code_start = (|| {
                for c in ["//", "#", "--"] {
                    if code_str.starts_with(c) {
                        return Ok(c)
                    }
                }
                return Err(anyhow::anyhow!("can't detect comment in text contract {}", &code_str[..10]));
            })()?;
            let mut code_comment = String::new();
            for l in code_str.lines() {
                if !l.starts_with(code_start) {
                    break;
                }
                code_comment.push_str(&l[code_start.len()..])
            }
            let runner_desc = serde_json::from_str(&code_comment)?;
            supervisor.api.get_runner(runner_desc)?
        };

    Ok(InitActions {
        code: code,
        actions: Arc::new(actions),
    })
}

pub fn run_with_api(mut api: Box<dyn RequiredApis>) -> Result<crate::vm::VMRunResult> {

    let mut entrypoint = b"call!".to_vec();
    let calldata = api.get_calldata()?;
    entrypoint.extend_from_slice(calldata.as_bytes());

    let init_data = api.get_initial_data()?;

    let supervisor = Arc::new(Mutex::new(vm::Supervisor::new(api)?));

    let (mut vm, instance) = {
        let supervisor_clone = supervisor.clone();
        let Ok(mut supervisor) = supervisor.lock() else { return Err(anyhow::anyhow!("can't lock supervisor")); };
        let init_actions = get_actions(&mut supervisor, &init_data)?;

        let essential_data = wasi::genlayer_sdk::EssentialGenlayerSdkData {
            conf: wasi::base::Config {
                is_deterministic: true,
                can_read_storage: true,
                can_write_storage: true
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
