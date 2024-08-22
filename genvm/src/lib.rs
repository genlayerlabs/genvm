mod driver;

pub mod vm;
pub mod wasi;
pub mod node_iface;

use anyhow::{Context as _, Result};
use core::str;
use std::{borrow::BorrowMut, path::Path, sync::Arc};

pub trait RequiredApis: node_iface::InitApi + node_iface::RunnerApi {}

fn link_wasm_into(supervisor: &mut vm::Supervisor, ret_vm: &mut vm::VM, contents: Arc<Vec<u8>>, debug_path: Option<String>) -> Result<wasmtime::Module> {
    let is_some = debug_path.is_some();
    let v = debug_path.unwrap_or_default();
    let debug_path = if is_some { Some(Path::new(&v[..])) } else { None };
    let prec = supervisor.cache_module(contents, debug_path)?;
    if ret_vm.is_det() {
        Ok(prec.det.clone())
    } else {
        Ok(prec.non_det.clone())
    }
}

fn instantiate_from_text(supervisor: &mut vm::Supervisor, api: &mut dyn RequiredApis, code: Arc<Vec<u8>>, ret_vm: &mut vm::VM) -> Result<wasmtime::Instance> {
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
    let runner = api.get_runner(runner_desc)?;
    let mut env = Vec::new();

    for act in runner {
        match act {
            node_iface::InitAction::MapFile { to, contents } => ret_vm.store.data_mut().genlayer_ctx_mut().preview1.map_file(&to, Arc::new(contents))?,
            node_iface::InitAction::MapCode { to } => ret_vm.store.data_mut().genlayer_ctx_mut().preview1.map_file(&to, code.clone())?,
            node_iface::InitAction::AddEnv { name, val } => env.push((name, val)),
            node_iface::InitAction::SetArgs { args } => ret_vm.store.data_mut().genlayer_ctx_mut().preview1.set_args(&args[..])?,
            node_iface::InitAction::LinkWasm { contents, debug_path } => { link_wasm_into(supervisor, ret_vm, Arc::new(contents), debug_path)?; },
            node_iface::InitAction::StartWasm { contents, debug_path } => {
                ret_vm.store.data_mut().genlayer_ctx_mut().preview1.set_env(&env[..])?;
                let module = link_wasm_into(supervisor, ret_vm, Arc::new(contents), debug_path)?;
                return ret_vm.linker.instantiate(ret_vm.store.borrow_mut() as &mut wasmtime::Store<_>, &module);
            },
        }
    }
    Err(anyhow::anyhow!("actions returned by runner do not have a start instruction"))
}

fn create_and_run_vm_for(supervisor: &mut vm::Supervisor, api: &mut dyn RequiredApis, code: Arc<Vec<u8>>, data: wasi::genlayer_sdk::EssentialGenlayerSdkData) -> Result<()> {
    let mut ret_vm = supervisor.spawn(data)?;

    let instance =
        if wasmparser::Parser::is_core_wasm(&code[..]) {
            let prec = supervisor.cache_module(code, Some(Path::new("/contract.wasm")))?;
            ret_vm.linker.instantiate(&mut ret_vm.store, &prec.det)
        } else {
            instantiate_from_text(supervisor, api, code, &mut ret_vm)
        }?;

    let func =
        instance.
            get_typed_func::<(), ()>(&mut ret_vm.store, "")
            .or_else(|_| instance.get_typed_func::<(), ()>(&mut ret_vm.store, "_start"))
            .with_context(|| "can't find entrypoint")?;
    func.call(&mut ret_vm.store, ())?;

    eprintln!("remaining fuel: {}", ret_vm.store.get_fuel().map(|s| s.to_string()).unwrap_or_else(|e| format!("<error> {}", e)));

    Ok(())
}

pub fn run_with_api(api: &mut dyn RequiredApis) -> Result<()> {
    let mut supervisor = vm::Supervisor::new()?;

    let init_data = api.get_initial_data()?;

    let code = api.get_code(&init_data.contract_account)?;

    let data = wasi::genlayer_sdk::EssentialGenlayerSdkData {
        conf: wasi::base::Config {
            is_deterministic: true,
            can_read_storage: true,
            can_write_storage: true
        },
        message_data: init_data,
    };

    create_and_run_vm_for(&mut supervisor, api, code, data)?;

    Ok(())
}
