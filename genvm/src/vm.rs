use core::str;
use std::{collections::HashMap, path::Path};

use serde::{Deserialize, Serialize};
use wasmtime::{Engine, Linker, Module, Store};

use crate::{runner::{self, InitAction}, wasi};
use anyhow::{Context, Result};
use std::sync::{Arc, Mutex};

#[derive(Serialize, Deserialize, Clone)]
pub struct RunnerComment {
    pub runner: Vec<String>,
}

#[derive(Clone)]
pub struct Host {
    genlayer_ctx: Arc<Mutex<wasi::Context>>,
}

impl Host {
    fn new(data: crate::wasi::genlayer_sdk::EssentialGenlayerSdkData) -> Host {
        return Host {
            genlayer_ctx: Arc::new(Mutex::new(wasi::Context::new(data))),
        };
    }
}

impl Host {
    pub fn genlayer_ctx_mut(&mut self) -> &mut wasi::Context {
        Arc::get_mut(&mut self.genlayer_ctx)
            .expect("wasmtime_wasi is not compatible with threads")
            .get_mut()
            .unwrap()
    }
}

pub struct PrecompiledModule {
    pub det: Module,
    pub non_det: Module,
}

pub struct Supervisor {
    det_engine: Engine,
    non_det_engine: Engine,
    cached_modules: HashMap<Arc<[u8]>, Arc<PrecompiledModule>>,
    pub api: Box<dyn crate::RequiredApis>,
    runner_cache: runner::RunnerReaderCache,
}

#[derive(Clone)]
pub struct InitActions {
    pub code: Arc<[u8]>,
    pub actions: Arc<Vec<InitAction>>,
}

pub struct VM {
    pub store: Store<Host>,
    pub linker: Linker<Host>,
    pub config_copy: wasi::base::Config,
    pub init_actions: InitActions,
}

pub use crate::node_iface::VMRunResult;

impl VM {
    pub fn is_det(&self) -> bool {
        self.config_copy.is_deterministic
    }

    pub fn run(&mut self, instance: &wasmtime::Instance) -> Result<VMRunResult> {
        let func = instance
            .get_typed_func::<(), ()>(&mut self.store, "")
            .or_else(|_| instance.get_typed_func::<(), ()>(&mut self.store, "_start"))
            .with_context(|| "can't find entrypoint")?;
        let res: VMRunResult = match func.call(&mut self.store, ()) {
            Ok(()) => VMRunResult::Return("".into()),
            Err(e) => {
                let res: Option<VMRunResult> = [
                    e.downcast_ref::<crate::wasi::preview1::I32Exit>()
                        .and_then(|v| {
                            if v.0 == 0 {
                                Some(VMRunResult::Return("".into()))
                            } else {
                                None
                            }
                        }),
                    e.downcast_ref::<crate::wasi::genlayer_sdk::Rollback>()
                        .map(|v| VMRunResult::Rollback(v.0.clone())),
                    e.downcast_ref::<crate::wasi::genlayer_sdk::ContractReturn>()
                        .map(|v| VMRunResult::Return(v.0.clone())),
                ]
                .into_iter()
                .fold(None, |x, y| if x.is_some() { x } else { y });
                res.unwrap_or(VMRunResult::Error(format!("{}", e)))
            }
        };
        Ok(res)
    }
}

impl Supervisor {
    pub fn new(api: Box<dyn crate::RequiredApis>) -> Result<Self> {
        let mut base_conf = wasmtime::Config::default();
        base_conf.cranelift_opt_level(wasmtime::OptLevel::None);
        //base_conf.cranelift_opt_level(wasmtime::OptLevel::Speed);
        base_conf.wasm_tail_call(true);
        base_conf.wasm_relaxed_simd(false);
        base_conf.cache_config_load_default()?;
        base_conf.consume_fuel(true);
        //base_conf.wasm_threads(false);
        //base_conf.wasm_reference_types(false);
        base_conf.wasm_simd(false);
        base_conf.relaxed_simd_deterministic(false);

        let mut det_conf = base_conf.clone();
        det_conf.async_support(false);
        det_conf.wasm_floats_enabled(false);

        let mut non_det_conf = base_conf.clone();
        non_det_conf.async_support(false);
        non_det_conf.wasm_floats_enabled(true);

        let det_engine = Engine::new(&det_conf)?;
        let non_det_engine = Engine::new(&non_det_conf)?;
        Ok(Self {
            det_engine,
            non_det_engine,
            cached_modules: HashMap::new(),
            api,
            runner_cache: runner::RunnerReaderCache::new(),
        })
    }

    pub fn cache_module(
        &mut self,
        module_bytes: Arc<[u8]>,
        path: Option<&Path>,
    ) -> Result<Arc<PrecompiledModule>> {
        let entry = self.cached_modules.entry(module_bytes.clone());
        match entry {
            std::collections::hash_map::Entry::Occupied(entry) => Ok(entry.get().clone()),
            std::collections::hash_map::Entry::Vacant(entry) => {
                let mut det_validator = wasmparser::Validator::new_with_features(
                    *self.det_engine.config().get_features(),
                );
                let mut non_det_validator = wasmparser::Validator::new_with_features(
                    *self.det_engine.config().get_features(),
                );
                det_validator.validate_all(&module_bytes[..])?;
                non_det_validator.validate_all(&module_bytes[..])?;
                let module_det = wasmtime::CodeBuilder::new(&self.det_engine)
                    .wasm_binary(&module_bytes[..], path)?
                    .compile_module()?;

                let module_non_det = wasmtime::CodeBuilder::new(&self.non_det_engine)
                    .wasm_binary(&module_bytes[..], path)?
                    .compile_module()?;
                let ret = PrecompiledModule {
                    det: module_det,
                    non_det: module_non_det,
                };
                Ok(entry.insert(Arc::new(ret)).clone())
            }
        }
    }

    pub fn spawn(
        &mut self,
        data: crate::wasi::genlayer_sdk::EssentialGenlayerSdkData,
    ) -> Result<VM> {
        let config_copy = data.conf.clone();
        let init_actions = data.init_actions.clone();

        let engine = if data.conf.is_deterministic {
            &self.det_engine
        } else {
            &self.non_det_engine
        };

        let init_gas = data.message_data.gas;
        let mut store = Store::new(&engine, Host::new(data));
        store.set_fuel(init_gas)?;

        let mut linker = Linker::new(engine);
        linker.allow_unknown_exports(false);
        linker.allow_shadowing(false);

        crate::wasi::add_to_linker_sync(&mut linker, |host: &mut Host| host.genlayer_ctx_mut())?;

        Ok(VM {
            store,
            linker,
            config_copy,
            init_actions,
        })
    }

    fn link_wasm_into(
        &mut self,
        ret_vm: &mut VM,
        contents: Arc<[u8]>,
        debug_path: Option<&str>,
    ) -> Result<wasmtime::Module> {
        let is_some = debug_path.is_some();
        let v = debug_path.clone().unwrap_or_default();
        let debug_path = if is_some {
            Some(Path::new(&v[..]))
        } else {
            None
        };
        let prec = self.cache_module(contents, debug_path)?;
        if ret_vm.is_det() {
            Ok(prec.det.clone())
        } else {
            Ok(prec.non_det.clone())
        }
    }

    pub fn apply_actions(&mut self, vm: &mut VM) -> Result<wasmtime::Instance> {
        let mut env = Vec::new();

        for act in vm.init_actions.actions.clone().iter() {
            match act {
                crate::runner::InitAction::MapFile { to, contents } => vm
                    .store
                    .data_mut()
                    .genlayer_ctx_mut()
                    .preview1
                    .map_file(&to, contents.clone())?,
                crate::runner::InitAction::MapCode { to } => vm
                    .store
                    .data_mut()
                    .genlayer_ctx_mut()
                    .preview1
                    .map_file(&to, vm.init_actions.code.clone())?,
                crate::runner::InitAction::AddEnv { name, val } => {
                    env.push((name.clone(), val.clone()))
                }
                crate::runner::InitAction::SetArgs { args } => vm
                    .store
                    .data_mut()
                    .genlayer_ctx_mut()
                    .preview1
                    .set_args(&args[..])?,
                crate::runner::InitAction::LinkWasm {
                    contents,
                    debug_path,
                } => {
                    let module = self.link_wasm_into(vm, contents.clone(), Some(debug_path))?;
                    let instance = vm.linker.instantiate(&mut vm.store, &module)?;
                    let name = module.name().ok_or(anyhow::anyhow!(
                        "can't link unnamed module {:?}",
                        &debug_path
                    ))?;
                    vm.linker.instance(&mut vm.store, name, instance)?;
                }
                crate::runner::InitAction::StartWasm {
                    contents,
                    debug_path,
                } => {
                    vm.store
                        .data_mut()
                        .genlayer_ctx_mut()
                        .preview1
                        .set_env(&env[..])?;
                    let module = self.link_wasm_into(vm, contents.clone(), Some(debug_path))?;
                    return vm.linker.instantiate(&mut vm.store, &module);
                }
            }
        }
        Err(anyhow::anyhow!(
            "actions returned by runner do not have a start instruction"
        ))
    }

    pub fn get_actions_for(
        &mut self,
        contract_account: &crate::node_iface::Address,
    ) -> Result<InitActions> {
        let code = self.api.get_code(contract_account)?;
        let actions = if wasmparser::Parser::is_core_wasm(&code[..]) {
            Vec::from([InitAction::StartWasm {
                contents: code.clone(),
                debug_path: "<contract>".into(),
            }])
        } else {
            let code_str = str::from_utf8(&code[..])?;
            let code_start = (|| {
                for c in ["//", "#", "--"] {
                    if code_str.starts_with(c) {
                        return Ok(c);
                    }
                }
                return Err(anyhow::anyhow!(
                    "can't detect comment in text contract {}",
                    &code_str[..10]
                ));
            })()?;
            let mut code_comment = String::new();
            for l in code_str.lines() {
                if !l.starts_with(code_start) {
                    break;
                }
                code_comment.push_str(&l[code_start.len()..])
            }
            let runner_desc: RunnerComment = serde_json::from_str(&code_comment)?;

            let mut runner = runner::RunnerReader::new()?;
            for c in &runner_desc.runner {
                runner.append(Arc::from(&c[..]), &mut self.runner_cache)?;
            }

            runner.get()?
        };

        Ok(InitActions {
            code,
            actions: Arc::new(actions),
        })
    }
}
