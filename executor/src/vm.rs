use core::str;
use std::{
    collections::{BTreeMap, HashMap},
    io::Write,
    path::Path,
    sync::atomic::AtomicU32,
};

use genvm_modules_common::interfaces::{llm_functions_api, web_functions_api};
use itertools::Itertools;
use wasmparser::WasmFeatures;
use wasmtime::{Engine, Linker, Module, Store};

use crate::{
    runner::{self, InitAction},
    wasi,
};
use anyhow::{Context, Result};
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub struct DecodeUtf8<I: Iterator<Item = u8>>(std::iter::Peekable<I>);

pub fn decode_utf8<I: IntoIterator<Item = u8>>(i: I) -> DecodeUtf8<I::IntoIter> {
    DecodeUtf8(i.into_iter().peekable())
}

#[derive(PartialEq, Debug)]
pub struct InvalidSequence(pub Vec<u8>);

impl<I: Iterator<Item = u8>> Iterator for DecodeUtf8<I> {
    type Item = Result<char, InvalidSequence>;
    #[inline]
    fn next(&mut self) -> Option<Result<char, InvalidSequence>> {
        let mut on_err: Vec<u8> = Vec::new();
        self.0.next().map(|b| {
            on_err.push(b);
            if b & 0x80 == 0 {
                Ok(b as char)
            } else {
                let l = (!b).leading_zeros() as usize; // number of bytes in UTF-8 representation
                if l < 2 || l > 6 {
                    return Err(InvalidSequence(on_err));
                };
                let mut x = (b as u32) & (0x7F >> l);
                for _ in 0..l - 1 {
                    match self.0.peek() {
                        Some(&b) if b & 0xC0 == 0x80 => {
                            on_err.push(b);
                            self.0.next();
                            x = (x << 6) | (b as u32) & 0x3F;
                        }
                        _ => return Err(InvalidSequence(on_err)),
                    }
                }
                match char::from_u32(x) {
                    Some(x) if l == x.len_utf8() => Ok(x),
                    _ => Err(InvalidSequence(on_err)),
                }
            }
        })
    }
}

pub enum RunOk {
    Return(Vec<u8>),
    Rollback(String),
}

pub type RunResult = Result<RunOk>;

impl RunOk {
    pub fn empty_return() -> Self {
        Self::Return([0].into())
    }

    pub fn as_bytes_iter<'a>(&'a self) -> impl Iterator<Item = u8> + use<'a> {
        match self {
            RunOk::Return(buf) => [0].into_iter().chain(buf.iter().cloned()),
            RunOk::Rollback(buf) => [1].into_iter().chain(buf.as_bytes().iter().cloned()),
        }
    }
}

impl std::fmt::Debug for RunOk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Return(r) => {
                let str = decode_utf8(r.iter().cloned())
                    .map(|r| match r {
                        Ok('\\') => "\\\\".into(),
                        Ok(c) if c.is_control() || c == '\n' || c == '\x07' => {
                            if c as u32 <= 255 {
                                format!("\\x{:02x}", c as u32)
                            } else {
                                format!("\\u{:04x}", c as u32)
                            }
                        }
                        Ok(c) => c.to_string(),
                        Err(InvalidSequence(seq)) => {
                            seq.iter().map(|c| format!("\\{:02x}", *c as u32)).join("")
                        }
                    })
                    .join("");
                f.write_fmt(format_args!("Return(\"{}\")", str))
            }
            Self::Rollback(r) => f.debug_tuple("Rollback").field(r).finish(),
        }
    }
}

#[derive(Clone)]
pub struct WasmContext {
    genlayer_ctx: Arc<Mutex<wasi::Context>>,
}

impl WasmContext {
    fn new(
        data: crate::wasi::genlayer_sdk::SingleVMData,
        shared_data: Arc<SharedData>,
    ) -> WasmContext {
        return WasmContext {
            genlayer_ctx: Arc::new(Mutex::new(wasi::Context::new(data, shared_data))),
        };
    }
}

impl WasmContext {
    pub fn genlayer_ctx_mut(&mut self) -> &mut wasi::Context {
        Arc::get_mut(&mut self.genlayer_ctx)
            .expect("wasmtime_wasi is not compatible with threads")
            .get_mut()
            .unwrap()
    }
}

pub struct SharedData {
    /// shared across all deterministic VMs
    pub nondet_call_no: AtomicU32,
    // rust doesn't have aliasing Arc constructor
    pub fuel_descriptor: Arc<wasmtime::FuelDescriptor>,
}

impl SharedData {
    fn new(total_gas: u64) -> Self {
        Self {
            nondet_call_no: 0.into(),
            fuel_descriptor: Arc::new(wasmtime::FuelDescriptor::new(total_gas)),
        }
    }
}

pub struct PrecompiledModule {
    pub det: Module,
    pub non_det: Module,
}

pub struct Modules {
    pub web: Box<dyn web_functions_api::Trait>,
    pub llm: Box<dyn llm_functions_api::Trait>,
}

// impl Drop for Modules {
//     fn drop(&mut self) {
//         eprintln!(
//             "{}",
//             std::fs::read_to_string(std::path::Path::new("/proc/self/maps"))
//                 .unwrap_or(String::new())
//         );
//     }
// }

pub struct Supervisor {
    pub modules: Modules,
    pub host: crate::Host,
    pub shared_data: Arc<SharedData>,
    pub fuel_desc: Arc<wasmtime::FuelDescriptor>,

    det_engine: Engine,
    non_det_engine: Engine,
    cached_modules: HashMap<Arc<[u8]>, Arc<PrecompiledModule>>,
    runner_cache: runner::RunnerReaderCache,
}

#[derive(Clone)]
pub struct InitActions {
    pub code: Arc<[u8]>,
    pub(crate) actions: Arc<Vec<InitAction>>,
}

pub struct VM {
    pub store: Store<WasmContext>,
    pub linker: Linker<WasmContext>,
    pub config_copy: wasi::base::Config,
    pub init_actions: InitActions,
}

impl VM {
    pub fn is_det(&self) -> bool {
        self.config_copy.is_deterministic
    }

    pub fn run(&mut self, instance: &wasmtime::Instance) -> RunResult {
        (|| {
            let Ok(lck) = self.store.data().genlayer_ctx.lock() else {
                return;
            };
            let mut stderr = std::io::stderr().lock();
            let _ = stderr.write(b"Spawning genvm\n");
            lck.preview1.log(&mut stderr);
            lck.genlayer_sdk.log(&mut stderr);
            let _ = stderr.flush();
        })();

        let func = instance
            .get_typed_func::<(), ()>(&mut self.store, "")
            .or_else(|_| instance.get_typed_func::<(), ()>(&mut self.store, "_start"))
            .with_context(|| "can't find entrypoint")?;
        let res: RunResult = match func.call(&mut self.store, ()) {
            Ok(()) => Ok(RunOk::empty_return()),
            Err(e) => {
                let res: Option<RunOk> = [
                    e.downcast_ref::<crate::wasi::preview1::I32Exit>()
                        .and_then(|v| {
                            if v.0 == 0 {
                                Some(RunOk::empty_return())
                            } else {
                                None
                            }
                        }),
                    e.downcast_ref::<crate::wasi::genlayer_sdk::Rollback>()
                        .map(|v| RunOk::Rollback(v.0.clone())),
                    e.downcast_ref::<crate::wasi::genlayer_sdk::ContractReturn>()
                        .map(|v| RunOk::Return(v.0.clone())),
                ]
                .into_iter()
                .fold(None, |x, y| if x.is_some() { x } else { y });
                res.map_or(Err(e), Ok)
            }
        };
        res
    }
}

impl Supervisor {
    pub fn new(modules: Modules, total_gas: u64, host: crate::Host) -> Result<Self> {
        let mut base_conf = wasmtime::Config::default();
        base_conf.cranelift_opt_level(wasmtime::OptLevel::None);
        //base_conf.cranelift_opt_level(wasmtime::OptLevel::Speed);
        base_conf.wasm_tail_call(true);
        base_conf.wasm_bulk_memory(true);
        base_conf.wasm_relaxed_simd(false);
        base_conf.wasm_simd(true);
        base_conf.wasm_relaxed_simd(false);
        base_conf.wasm_feature(WasmFeatures::BULK_MEMORY, true);
        base_conf.wasm_feature(WasmFeatures::REFERENCE_TYPES, false);
        base_conf.wasm_feature(WasmFeatures::SIGN_EXTENSION, true);
        base_conf.wasm_feature(WasmFeatures::MUTABLE_GLOBAL, true);
        base_conf.wasm_feature(WasmFeatures::SATURATING_FLOAT_TO_INT, false);
        base_conf.wasm_feature(WasmFeatures::MULTI_VALUE, true);

        match directories_next::ProjectDirs::from("", "yagerai", "genvm") {
            None => {
                base_conf.disable_cache();
            }
            Some(dirs) => {
                let cache_dir = dirs.cache_dir().join("modules");
                let cache_conf: wasmtime_cache::CacheConfig =
                    serde_json::from_value(serde_json::Value::Object(
                        [
                            ("enabled".into(), serde_json::Value::Bool(true)),
                            (
                                "directory".into(),
                                cache_dir.into_os_string().into_string().unwrap().into(),
                            ),
                        ]
                        .into_iter()
                        .collect(),
                    ))?;
                base_conf.cache_config_set(cache_conf)?;
            }
        }

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
        let shared_data = Arc::new(SharedData::new(total_gas));
        let fuel_desc = shared_data.fuel_descriptor.clone();
        Ok(Self {
            det_engine,
            non_det_engine,
            cached_modules: HashMap::new(),
            runner_cache: runner::RunnerReaderCache::new(),
            modules,
            host,
            shared_data,
            fuel_desc,
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
                // FIXME: find source of this. why call_indirect requires tables?
                let add_features =
                    WasmFeatures::REFERENCE_TYPES.bits() | WasmFeatures::FLOATS.bits();

                let det_features = self.det_engine.config().get_features().bits() | add_features;

                let non_det_features =
                    self.non_det_engine.config().get_features().bits() | add_features;

                let mut det_validator = wasmparser::Validator::new_with_features(
                    WasmFeatures::from_bits(det_features).unwrap(),
                );
                let mut non_det_validator = wasmparser::Validator::new_with_features(
                    WasmFeatures::from_bits(non_det_features).unwrap(),
                );
                det_validator
                    .validate_all(&module_bytes[..])
                    .with_context(|| {
                        format!(
                            "validating {}",
                            &String::from_utf8_lossy(&module_bytes[..10.min(module_bytes.len())])
                        )
                    })?;
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

    pub fn spawn(&mut self, data: crate::wasi::genlayer_sdk::SingleVMData) -> Result<VM> {
        let config_copy = data.conf.clone();
        let init_actions = data.init_actions.clone();

        let engine = if data.conf.is_deterministic {
            &self.det_engine
        } else {
            &self.non_det_engine
        };

        let store = Store::new(
            &engine,
            self.fuel_desc.clone(),
            WasmContext::new(data, self.shared_data.clone()),
        );

        let mut linker = Linker::new(engine);
        linker.allow_unknown_exports(false);
        linker.allow_shadowing(false);

        crate::wasi::add_to_linker_sync(&mut linker, |host: &mut WasmContext| {
            host.genlayer_ctx_mut()
        })?;

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
        let precompiled = self
            .cache_module(contents, debug_path)
            .with_context(|| format!("caching {:?}", &debug_path))?;
        if ret_vm.is_det() {
            Ok(precompiled.det.clone())
        } else {
            Ok(precompiled.non_det.clone())
        }
    }

    pub fn apply_actions(&mut self, vm: &mut VM) -> Result<wasmtime::Instance> {
        let mut env = BTreeMap::new();

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
                crate::runner::InitAction::AddEnv { name, val } => match env.entry(name.clone()) {
                    std::collections::btree_map::Entry::Vacant(vacant_entry) => {
                        vacant_entry.insert(val.clone());
                    }
                    std::collections::btree_map::Entry::Occupied(mut occupied_entry) => {
                        occupied_entry.get_mut().push_str(":");
                        occupied_entry.get_mut().push_str(val);
                    }
                },
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
                    match instance.get_typed_func::<(), ()>(&mut vm.store, "_initialize") {
                        Err(_) => {}
                        Ok(func) => {
                            func.call(&mut vm.store, ())?;
                        }
                    }
                }
                crate::runner::InitAction::StartWasm {
                    contents,
                    debug_path,
                } => {
                    let env: Vec<(String, String)> = env.into_iter().collect();
                    vm.store
                        .data_mut()
                        .genlayer_ctx_mut()
                        .preview1
                        .set_env(&env)?;
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
        contract_account: &crate::AccountAddress,
    ) -> Result<InitActions> {
        let code = self.host.get_code(contract_account)?;
        let actions = if wasmparser::Parser::is_core_wasm(&code[..]) {
            Vec::from([InitAction::StartWasm {
                contents: code.clone(),
                debug_path: "<contract>".into(),
            }])
        } else if let Ok(mut as_contr) = zip::ZipArchive::new(std::io::Cursor::new(&code)) {
            let mut runner = runner::RunnerReader::new()?;
            runner.append_archive("<contract>", &mut as_contr, &mut self.runner_cache)?;
            runner.get()?
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
                    &code_str[..10.min(code_str.len())]
                ));
            })()?;
            let mut code_comment = String::new();
            for l in code_str.lines() {
                if !l.starts_with(code_start) {
                    break;
                }
                code_comment.push_str(&l[code_start.len()..])
            }

            let mut buf = [0; 4096];
            let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut buf[..]));
            zip.start_file(
                "runner.json",
                zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Stored),
            )?;
            zip.write_all(code_comment.as_bytes())?;
            zip.finish()?;

            let mut zip = zip::ZipArchive::new(std::io::Cursor::new(&mut buf[..]))?;
            let mut runner = runner::RunnerReader::new()?;
            runner.append_archive("<contract>", &mut zip, &mut self.runner_cache)?;
            runner.get()?
        };

        Ok(InitActions {
            code,
            actions: Arc::new(actions),
        })
    }
}
