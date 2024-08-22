use std::{collections::HashMap, path::Path};

use wasmtime::{Module, Engine, Store, Linker};

use std::sync::{Arc, Mutex};
use crate::wasi;
use anyhow::Result;

#[derive(Clone)]
pub struct Host {
    genlayer_ctx: Arc<Mutex<wasi::Context>>,
}

impl Host {
    fn new(data: crate::wasi::genlayer_sdk::EssentialGenlayerSdkData) -> Host {
        return Host{
            genlayer_ctx: Arc::new(Mutex::new(wasi::Context::new(data))),
        }
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
    det_validator: wasmparser::Validator,
    non_det_engine: Engine,
    non_det_validator: wasmparser::Validator,
    cached_modules: HashMap<Arc<Vec<u8>>, Arc<PrecompiledModule>>,
}

pub struct VM {
    pub store: Store<Host>,
    pub linker: Linker<Host>,
    pub config_copy: wasi::base::Config,
}

impl VM {
    pub fn is_det(&self) -> bool {
        self.config_copy.is_deterministic
    }
}

impl Supervisor {
    pub fn new() -> Result<Self> {
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
        let non_det_engine= Engine::new(&non_det_conf)?;
        let det_validator = wasmparser::Validator::new_with_features(*det_engine.config().get_features());
        let non_det_validator = wasmparser::Validator::new_with_features(*det_engine.config().get_features());
        Ok(Self {
            det_engine,
            det_validator,
            non_det_engine,
            non_det_validator,
            cached_modules: HashMap::new(),
        })
    }

    pub fn cache_module(&mut self, module_bytes: Arc<Vec<u8>>, path: Option<&Path>) -> Result<Arc<PrecompiledModule>> {
        let entry = self.cached_modules.entry(module_bytes.clone());
        match entry {
            std::collections::hash_map::Entry::Occupied(entry) => Ok(entry.get().clone()),
            std::collections::hash_map::Entry::Vacant(entry) => {
                self.det_validator.validate_all(&module_bytes[..])?;
                self.non_det_validator.validate_all(&module_bytes[..])?;
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
            },
        }
    }

    pub fn spawn(&mut self, data: crate::wasi::genlayer_sdk::EssentialGenlayerSdkData) -> Result<VM> {
        let config_copy = data.conf.clone();

        let engine = if data.conf.is_deterministic { &self.det_engine } else { &self.non_det_engine };

        let init_gas = data.message_data.gas;
        let mut store = Store::new(&engine, Host::new(data));
        store.set_fuel(init_gas)?;

        let mut linker = Linker::new(engine);
        linker.allow_unknown_exports(false);
        linker.allow_shadowing(false);

        crate::wasi::add_to_linker_sync(&mut linker, |host: &mut Host| {
            host.genlayer_ctx_mut()
        })?;

        Ok(VM {
            store,
            linker,
            config_copy,
        })
    }
}
