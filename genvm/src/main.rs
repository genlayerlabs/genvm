mod driver;
pub(crate) mod vm;
pub(crate) mod wasi;

pub(crate) mod node_iface;

use core::str;
use std::{borrow::BorrowMut, io::Read, ops::DerefMut, path::Path, sync::{Arc, Mutex}};

use anyhow::{Context, Result};
use clap::Parser;
use wasmtime::*;


pub trait Oao<T: 'static, U>: Send + Sync + Clone + 'static {
    fn build<'a>(&self) -> impl Fn(wasmtime::StoreContextMut<'a, T>) -> U;
}

mod test_node_iface_impl {
    use std::sync::Arc;

    use crate::node_iface::{self, Address};
    use anyhow::Result;

    pub fn make_addr_from_byte(b: u8) -> Address {
        let mut r = [0;32];
        r[0] = b;
        Address(r)
    }

    pub struct TestApi {}

    impl node_iface::RunnerApi for TestApi {
        fn get_runner(&mut self, desc: node_iface::RunnerDescription) -> anyhow::Result<Vec<node_iface::InitAction>> {
            if desc.lang != "python" {
                return Err(anyhow::anyhow!("unsupported language"));
            }
            let rp = std::fs::read("testdata/genvm-python.wasm")?;
            Ok(Vec::from([
                node_iface::InitAction::AddEnv { name: "pwd".into(), val: "/".into() },
                node_iface::InitAction::MapCode { to: "/contract.py".into() },
                node_iface::InitAction::SetArgs { args: Vec::from(["py".into(), "contract.py".into()]) },
                node_iface::InitAction::StartWasm { contents: rp, debug_path: Some("genvm-python.wasm".into()) }
            ]))
        }
    }

    #[allow(unused_variables)]
    impl node_iface::InitApi for TestApi {
        fn get_initial_data(&mut self) -> Result<node_iface::MessageData> {
            Ok(node_iface::MessageData {
                initial_gas: u64::max_value(),
                account: make_addr_from_byte(1),
                value: None,
                calldata: String::from(r#"{"method": "init", "args": []}"#),
            })
        }

        fn get_code(&mut self, account: &node_iface::Address) -> Result<Arc<Vec<u8>>> {
            if *account == make_addr_from_byte(1) {
                Ok(Arc::new(std::fs::read("testdata/0.py")?))
            } else {
                Err(anyhow::anyhow!("unknown account"))
            }
        }
    }
}

trait mock_apis: node_iface::InitApi + node_iface::RunnerApi {}

impl mock_apis for test_node_iface_impl::TestApi {}

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

fn instantiate_from_text(supervisor: &mut vm::Supervisor, api: &mut dyn mock_apis, code: Arc<Vec<u8>>, ret_vm: &mut vm::VM) -> Result<wasmtime::Instance> {
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
                return ret_vm.linker.instantiate(ret_vm.store.borrow_mut() as &mut Store<_>, &module);
            },
        }
    }
    Err(anyhow::anyhow!("actions returned by runner do not have a start instruction"))
}

fn with_state<R, T>(data: Arc<Mutex<T>>, f: impl FnOnce(&mut T) -> R) -> R {
    let state = &mut data.lock().expect("Could not lock mutex");
    f(state)
}

fn create_and_run_vm_for(supervisor: &mut vm::Supervisor, api: &mut dyn mock_apis, code: Arc<Vec<u8>>, data: wasi::genlayer_sdk::EssentialGenlayerSdkData) -> Result<()> {
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

fn run_with_api(api: &mut dyn mock_apis) -> Result<()> {
    let mut supervisor = vm::Supervisor::new()?;

    let init_data = api.get_initial_data()?;

    let code = api.get_code(&init_data.account)?;

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

fn main() -> Result<()> {
    let mut node_api = test_node_iface_impl::TestApi {};
    run_with_api(&mut node_api)?;
    Ok(())
}
