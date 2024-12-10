use core::str;
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    io::{Read, Write},
    sync::atomic::AtomicU32,
};

use base64::Engine as _;
use genvm_modules_common::interfaces::{llm_functions_api, web_functions_api};
use itertools::Itertools;
use once_cell::sync::Lazy;
use wasmparser::WasmFeatures;
use wasmtime::{Engine, Linker, Module, Store};
use zip::ZipArchive;

use crate::{
    caching,
    runner::{self, InitAction, WasmMode},
    string_templater, wasi,
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
    ContractError(String, Option<anyhow::Error>),
}

pub type RunResult = Result<RunOk>;

impl RunOk {
    pub fn empty_return() -> Self {
        Self::Return([0].into())
    }

    pub fn as_bytes_iter<'a>(&'a self) -> impl Iterator<Item = u8> + use<'a> {
        use crate::host::ResultCode;
        match self {
            RunOk::Return(buf) => [ResultCode::Return as u8]
                .into_iter()
                .chain(buf.iter().cloned()),
            RunOk::Rollback(buf) => [ResultCode::Rollback as u8]
                .into_iter()
                .chain(buf.as_bytes().iter().cloned()),
            RunOk::ContractError(buf, _) => [ResultCode::ContractError as u8]
                .into_iter()
                .chain(buf.as_bytes().iter().cloned()),
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
            Self::ContractError(r, _) => f.debug_tuple("ContractError").field(r).finish(),
        }
    }
}

#[derive(Clone)]
pub struct WasmContext {
    genlayer_ctx: Arc<Mutex<wasi::Context>>,
    limits: wasmtime::StoreLimits,
}

impl WasmContext {
    fn new(
        data: crate::wasi::genlayer_sdk::SingleVMData,
        shared_data: Arc<SharedData>,
    ) -> WasmContext {
        WasmContext {
            genlayer_ctx: Arc::new(Mutex::new(wasi::Context::new(data, shared_data))),
            limits: wasmtime::StoreLimitsBuilder::new()
                .memories(100)
                .memory_size(2usize << 30)
                .instances(1000)
                .tables(1000)
                .table_elements(1usize << 20)
                .build(),
        }
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
    pub should_exit: Arc<AtomicU32>,
    pub is_sync: bool,
}

impl SharedData {
    fn new(is_sync: bool) -> Self {
        Self {
            nondet_call_no: 0.into(),
            should_exit: Arc::from(AtomicU32::from(0)),
            is_sync,
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

    engines: Engines,
    cached_modules: HashMap<Arc<[u8]>, Arc<PrecompiledModule>>,
    runner_cache: runner::RunnerReaderCache,
}

pub struct VM {
    pub store: Store<WasmContext>,
    pub linker: Arc<Mutex<Linker<WasmContext>>>,
    pub config_copy: wasi::base::Config,
}

struct ApplyActionCtx {
    env: BTreeMap<String, String>,
    visited: BTreeSet<symbol_table::GlobalSymbol>,
    contract_id: symbol_table::GlobalSymbol,
}

impl VM {
    pub fn is_det(&self) -> bool {
        self.config_copy.is_deterministic
    }

    pub fn run(&mut self, instance: &wasmtime::Instance) -> RunResult {
        if let Ok(lck) = self.store.data().genlayer_ctx.lock() {
            log::info!(target: "vm", method = "run", wasi_preview1: serde = lck.preview1.log(), genlayer_sdk: serde = lck.genlayer_sdk.log(); "");
        }

        let func = instance
            .get_typed_func::<(), ()>(&mut self.store, "")
            .or_else(|_| instance.get_typed_func::<(), ()>(&mut self.store, "_start"))
            .with_context(|| "can't find entrypoint")?;
        log::info!(target: "vm", event = "execution start"; "");
        let time_start = std::time::Instant::now();
        let res = func.call(&mut self.store, ());
        log::info!(target: "vm", event = "execution finished", duration:? = time_start.elapsed(); "");
        let res: RunResult = match res {
            Ok(()) => Ok(RunOk::empty_return()),
            Err(e) => {
                let res: Option<RunOk> = [
                    e.downcast_ref::<crate::wasi::preview1::I32Exit>()
                        .and_then(|v| {
                            if v.0 == 0 {
                                Some(RunOk::empty_return())
                            } else {
                                Some(RunOk::ContractError(format!("exit_code {}", v.0), None))
                            }
                        }),
                    e.downcast_ref::<wasmtime::Trap>()
                        .map(|v| RunOk::ContractError(format!("wasm_trap {v:?}"), None)),
                    e.downcast_ref::<crate::errors::ContractError>()
                        .map(|v| RunOk::ContractError(v.0.clone(), None)),
                    e.downcast_ref::<crate::errors::Rollback>()
                        .map(|v| RunOk::Rollback(v.0.clone())),
                    e.downcast_ref::<crate::wasi::genlayer_sdk::ContractReturn>()
                        .map(|v| RunOk::Return(v.0.clone())),
                ]
                .into_iter()
                .fold(None, |x, y| if x.is_some() { x } else { y });
                res.map_or(Err(e), Ok)
            }
        };
        match &res {
            Ok(RunOk::Return(_)) => {
                log::info!(target: "vm", event = "execution result unwrapped", result = "Return"; "")
            }
            Ok(RunOk::Rollback(_)) => {
                log::info!(target: "vm", event = "execution result unwrapped", result = "Rollback"; "")
            }
            Ok(RunOk::ContractError(e, cause)) => {
                log::info!(target: "vm", event = "execution result unwrapped", result = format!("ContractError({e})"), cause:? = cause; "")
            }
            Err(_) => {
                log::info!(target: "vm", event = "execution result unwrapped", result = "Error"; "")
            }
        };
        res
    }
}

pub struct Engines {
    pub det: Engine,
    pub non_det: Engine,
}

impl Engines {
    pub fn create(config_base: impl FnOnce(&mut wasmtime::Config) -> Result<()>) -> Result<Self> {
        let mut base_conf = wasmtime::Config::default();

        base_conf.debug_info(true);
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

        base_conf.consume_fuel(false);
        //base_conf.wasm_threads(false);
        //base_conf.wasm_reference_types(false);
        base_conf.wasm_simd(false);
        base_conf.relaxed_simd_deterministic(false);

        base_conf.cranelift_opt_level(wasmtime::OptLevel::None);
        config_base(&mut base_conf)?;

        let mut det_conf = base_conf.clone();
        det_conf.async_support(false);
        det_conf.wasm_floats_enabled(false);
        det_conf.cranelift_nan_canonicalization(true);

        let mut non_det_conf = base_conf.clone();
        non_det_conf.async_support(false);
        non_det_conf.wasm_floats_enabled(true);

        let det_engine = Engine::new(&det_conf)?;
        let non_det_engine = Engine::new(&non_det_conf)?;
        Ok(Self {
            det: det_engine,
            non_det: non_det_engine,
        })
    }
}

#[derive(Clone, Debug)]
pub struct WasmFileDesc {
    pub contents: Arc<[u8]>,
    pub runner_id: symbol_table::GlobalSymbol,
    pub path_in_arch: symbol_table::GlobalSymbol,
}

impl WasmFileDesc {
    pub fn debug_path(&self) -> String {
        [self.runner_id.as_str(), self.path_in_arch.as_str()].join("")
    }

    pub fn is_special(&self) -> bool {
        self.runner_id.as_str().starts_with("<")
    }
}

impl Supervisor {
    pub fn new(modules: Modules, mut host: crate::Host, is_sync: bool) -> Result<Self> {
        let engines = Engines::create(|base_conf| {
            match Lazy::force(&caching::CACHE_DIR) {
                None => {
                    base_conf.disable_cache();
                }
                Some(cache_dir) => {
                    let mut cache_dir = cache_dir.clone();
                    cache_dir.push("wasmtime");
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
            Ok(())
        });
        let engines = match engines {
            Ok(engines) => engines,
            Err(e) => {
                let err = Err(e);
                host.consume_result(&err)?;
                return Err(err.unwrap_err());
            }
        };
        let shared_data = Arc::new(SharedData::new(is_sync));
        Ok(Self {
            engines,
            cached_modules: HashMap::new(),
            runner_cache: runner::RunnerReaderCache::new()?,
            modules,
            host,
            shared_data,
        })
    }

    pub fn cache_module(&mut self, data: &WasmFileDesc) -> Result<Arc<PrecompiledModule>> {
        let entry = self.cached_modules.entry(data.contents.clone());
        match entry {
            std::collections::hash_map::Entry::Occupied(entry) => {
                log::debug!(target: "cache", cache_method = "rt", path = data.debug_path(); "using rt cached");
                Ok(entry.get().clone())
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                // FIXME: find source of this. why call_indirect requires tables?
                let add_features =
                    WasmFeatures::REFERENCE_TYPES.bits() | WasmFeatures::FLOATS.bits();

                let det_features = self.engines.det.config().get_features().bits() | add_features;

                let non_det_features =
                    self.engines.non_det.config().get_features().bits() | add_features;

                let mut det_validator = wasmparser::Validator::new_with_features(
                    WasmFeatures::from_bits(det_features).unwrap(),
                );
                let mut non_det_validator = wasmparser::Validator::new_with_features(
                    WasmFeatures::from_bits(non_det_features).unwrap(),
                );
                det_validator
                    .validate_all(&data.contents[..])
                    .with_context(|| {
                        format!(
                            "validating {}",
                            &String::from_utf8_lossy(&data.contents[..10.min(data.contents.len())])
                        )
                    })?;
                non_det_validator.validate_all(&data.contents[..])?;

                let debug_path = data.debug_path();

                let compile_here = || -> Result<PrecompiledModule> {
                    log::info!(target: "cache", cache_method = "compiling", status = "start", path = debug_path; "");

                    let start_time = std::time::Instant::now();
                    let module_det = wasmtime::CodeBuilder::new(&self.engines.det)
                        .wasm_binary(&data.contents[..], Some(std::path::Path::new(&debug_path)))?
                        .compile_module()?;

                    let module_non_det = wasmtime::CodeBuilder::new(&self.engines.non_det)
                        .wasm_binary(&data.contents[..], Some(std::path::Path::new(&debug_path)))?
                        .compile_module()?;
                    log::info!(target: "cache", cache_method = "compiling", status = "done", duration:? = start_time.elapsed(), path = debug_path; "");
                    Ok(PrecompiledModule {
                        det: module_det,
                        non_det: module_non_det,
                    })
                };

                let get_from_runner = || -> Result<PrecompiledModule> {
                    if data.is_special() {
                        anyhow::bail!("special runners are not supported");
                    }
                    let (id, hash) = runner::verify_runner(data.runner_id.as_str())?;

                    let path_in_arch = data.path_in_arch.as_str();
                    let mut result_zip_path = match Lazy::force(&caching::PRECOMPILE_DIR) {
                        Some(v) => v,
                        None => anyhow::bail!("cache is absent"),
                    }
                    .clone();

                    let hash = format!("{hash}.zip");

                    result_zip_path.push(id);
                    result_zip_path.push(&hash);
                    let mut zip = ZipArchive::new(std::fs::File::open(&result_zip_path)?)?;

                    let mut process_single = |suff: &str, engine: &Engine| -> Result<Module> {
                        let mut buf = Vec::new();
                        let mut path_in_arch = String::from(path_in_arch);
                        path_in_arch.push_str(suff);
                        zip.by_name(&path_in_arch)?.read_to_end(&mut buf)?;
                        Ok(unsafe { Module::deserialize(engine, &buf)? })
                    };

                    let det = process_single(
                        caching::DET_NON_DET_PRECOMPILED_SUFFIX.det,
                        &self.engines.det,
                    )?;
                    let non_det = process_single(
                        caching::DET_NON_DET_PRECOMPILED_SUFFIX.non_det,
                        &self.engines.non_det,
                    )?;

                    log::debug!(target: "cache", cache_method = "precompiled", path = data.debug_path(); "using precompiled");

                    Ok(PrecompiledModule { det, non_det })
                };

                let ret = get_from_runner().or_else(|_e| compile_here())?;

                Ok(entry.insert(Arc::new(ret)).clone())
            }
        }
    }

    pub fn spawn(&mut self, data: crate::wasi::genlayer_sdk::SingleVMData) -> Result<VM> {
        let config_copy = data.conf.clone();

        let engine = if data.conf.is_deterministic {
            &self.engines.det
        } else {
            &self.engines.non_det
        };

        let mut store = Store::new(
            &engine,
            WasmContext::new(data, self.shared_data.clone()),
            self.shared_data.should_exit.clone(),
        );

        store.limiter(|ctx| &mut ctx.limits);

        let linker_shared = Arc::new(Mutex::new(Linker::new(engine)));
        let linker_shared_cloned = linker_shared.clone();
        let Ok(ref mut linker) = linker_shared_cloned.lock() else {
            panic!();
        };
        linker.allow_unknown_exports(false);
        linker.allow_shadowing(false);

        crate::wasi::add_to_linker_sync(
            linker,
            linker_shared.clone(),
            |host: &mut WasmContext| host.genlayer_ctx_mut(),
        )?;

        Ok(VM {
            store,
            linker: linker_shared,
            config_copy,
        })
    }

    fn link_wasm_into(&mut self, ret_vm: &mut VM, data: &WasmFileDesc) -> Result<wasmtime::Module> {
        let precompiled = self
            .cache_module(data)
            .with_context(|| format!("caching {:?}", data.debug_path()))?;
        if ret_vm.is_det() {
            Ok(precompiled.det.clone())
        } else {
            Ok(precompiled.non_det.clone())
        }
    }

    fn apply_action_recursive(
        &mut self,
        vm: &mut VM,
        ctx: &mut ApplyActionCtx,
        action: &InitAction,
        current: symbol_table::GlobalSymbol,
    ) -> Result<Option<wasmtime::Instance>> {
        match action {
            InitAction::MapFile { to, file } => {
                if file.as_str().ends_with("/") {
                    for name in self.runner_cache.get_unsafe(current).get_all_names()?
                        .iter()
                        .cloned()
                        .filter(|name| !name.as_str().ends_with("/") && name.as_str().starts_with(file.as_str()))
                    {
                        let file_contents = self.runner_cache.get_unsafe(current).get_file(name)?;
                        let mut name_in_fs = to.clone();
                        if !name_in_fs.ends_with("/") {
                            name_in_fs.push('/');
                        }
                        name_in_fs.push_str(&name.as_str()[file.as_str().len()..]);
                        vm.store
                            .data_mut()
                            .genlayer_ctx_mut()
                            .preview1
                            .map_file(&name_in_fs, file_contents)?;
                    }
                } else {
                    vm.store
                    .data_mut()
                    .genlayer_ctx_mut()
                    .preview1
                    .map_file(&to, self.runner_cache.get_unsafe(current).get_file(*file)?)?;
                }
                Ok(None)
            }
            InitAction::AddEnv { name, val } => {
                let new_val = string_templater::patch_str(&ctx.env, val)?;
                ctx.env.insert(name.clone(), new_val);
                Ok(None)
            }
            InitAction::SetArgs(args) => {
                vm.store
                    .data_mut()
                    .genlayer_ctx_mut()
                    .preview1
                    .set_args(&args[..])?;
                Ok(None)
            }
            InitAction::LinkWasm(path) => {
                let path = *path;
                let contents = self.runner_cache.get_unsafe(current).get_file(path)?;
                let module = self.link_wasm_into(vm, &WasmFileDesc {
                    contents,
                    runner_id: current,
                    path_in_arch: path,
                })?;
                let instance = {
                    let Ok(ref mut linker) = vm.linker.lock() else {
                        panic!();
                    };
                    let instance = linker.instantiate(&mut vm.store, &module)?;
                    let name = module.name().ok_or(anyhow::anyhow!(
                        "can't link unnamed module {:?}",
                        current
                    )).map_err(|e| crate::errors::ContractError("invalid_wasm".into(), Some(e)))?;
                    linker.instance(&mut vm.store, name, instance)?;
                    instance
                };
                match instance.get_typed_func::<(), ()>(&mut vm.store, "_initialize") {
                    Err(_) => {}
                    Ok(func) => {
                        log::info!(target: "rt", method = "call_initialize", runner = self.runner_cache.get_unsafe(current).runner_id().as_str(), path = path.as_str(); "");
                        func.call(&mut vm.store, ())?;
                    }
                }
                Ok(None)
            }
            InitAction::StartWasm(path) => {
                let path = *path;
                let env: Vec<(String, String)> = ctx
                    .env
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                vm.store
                    .data_mut()
                    .genlayer_ctx_mut()
                    .preview1
                    .set_env(&env)?;
                let contents = self.runner_cache.get_unsafe(current).get_file(path)?;
                let module = self.link_wasm_into(vm, &WasmFileDesc {
                    contents,
                    runner_id: current,
                    path_in_arch: path,
                })?;
                let Ok(ref mut linker) = vm.linker.lock() else {
                    panic!();
                };
                Ok(Some(linker.instantiate(&mut vm.store, &module)?))
            }
            InitAction::When { cond, action } => {
                if (*cond == WasmMode::Det) != vm.is_det() {
                    return Ok(None);
                }
                self.apply_action_recursive(vm, ctx, action, current)
            }
            InitAction::Seq(vec) => {
                for act in vec {
                    match self.apply_action_recursive(vm, ctx, act, current)? {
                        Some(x) => return Ok(Some(x)),
                        None => {}
                    }
                }
                Ok(None)
            }
            InitAction::With { runner: id, action } => {
                if id.as_str() == "<contract>" {
                    return self.apply_action_recursive(vm, ctx, action, ctx.contract_id)
                }
                let mut path = std::path::PathBuf::from(self.runner_cache.path());
                let make_new_runner = || {
                    let (runner_id, runner_hash) = runner::verify_runner(id.as_str())?;

                    path.push(runner_id);
                    let mut fname = runner_hash.to_owned();
                    fname.push_str(".zip");
                    path.push(fname);

                    let contents = std::fs::read(&path).with_context(|| format!("reading {:?}", path))?;
                    Ok(zip::ZipArchive::new(std::io::Cursor::new(Arc::from(
                        contents,
                    )))?)
                };
                let _ = self.runner_cache.get_or_create(*id, make_new_runner)?;
                self.apply_action_recursive(vm, ctx, action, *id)
            }
            InitAction::Depends(id) => {
                if !ctx.visited.insert(*id) {
                    return Ok(None)
                }
                let mut path = std::path::PathBuf::from(self.runner_cache.path());
                let make_new_runner = || {
                    let (runner_id, runner_hash) = runner::verify_runner(id.as_str())?;

                    path.push(runner_id);
                    let mut fname = runner_hash.to_owned();
                    fname.push_str(".zip");
                    path.push(fname);

                    let contents = std::fs::read(&path).with_context(|| format!("reading {:?}", path))?;
                    Ok(zip::ZipArchive::new(std::io::Cursor::new(Arc::from(
                        contents,
                    )))?)
                };
                let new_arch = self.runner_cache.get_or_create(*id, make_new_runner)?;
                let new_action = new_arch.get_actions()?;
                self.apply_action_recursive(vm, ctx, &new_action, *id)
            }
        }
    }

    fn code_to_archive(code: Arc<[u8]>) -> Result<zip::ZipArchive<std::io::Cursor<Arc<[u8]>>>> {
        if let Ok(as_zip) = zip::ZipArchive::new(std::io::Cursor::new(code.clone())) {
            return Ok(as_zip);
        }
        let buf = std::io::Cursor::new(Vec::new());
        let mut zip = zip::ZipWriter::new(buf);

        if wasmparser::Parser::is_core_wasm(&code[..]) {
            zip.start_file(
                "runner.json",
                zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Stored),
            )?;
            zip.write_all("{ \"StartWasm\": \"file.wasm\" }".as_bytes())?;
            zip.start_file(
                "file.wasm",
                zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Stored),
            )?;
            zip.write_all(&code)?;
        } else  {
            let code_str = str::from_utf8(&code).map_err(|e| crate::errors::ContractError(
                "invalid_contract".into(),
                Some(anyhow::Error::from(e)),
            ))?;
            let code_start = (|| {
                for c in ["//", "#", "--"] {
                    if code_str.starts_with(c) {
                        return Ok(c);
                    }
                }
                return Err(crate::errors::ContractError(
                    "no_runner_comment".into(),
                    None,
                ));
            })()?;
            let mut code_comment = String::new();
            for l in code_str.lines() {
                if !l.starts_with(code_start) {
                    break;
                }
                code_comment.push_str(&l[code_start.len()..])
            }

            zip.start_file(
                "runner.json",
                zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Stored),
            )?;
            zip.write_all(code_comment.as_bytes())?;

            zip.start_file(
                "file",
                zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Stored),
            )?;
            zip.write_all(code_str.as_bytes())?;
        }

        let zip = zip.finish()?;
        Ok(zip::ZipArchive::new(std::io::Cursor::new(Arc::from(zip.into_inner())))?)
    }

    pub fn apply_contract_actions(&mut self, vm: &mut VM) -> Result<wasmtime::Instance> {
        let contract_address = {
            let lock = vm.store.data().genlayer_ctx.lock().unwrap();
            lock.genlayer_sdk.data.message_data.contract_account
        };

        let mut contract_id = String::from("<contract>:");
        contract_id.push_str(&base64::prelude::BASE64_STANDARD.encode(&contract_address.raw()));
        let contract_id = symbol_table::GlobalSymbol::from(&contract_id);

        let provide_arch = || {
            let code = self.host.get_code(&contract_address)?;
            Self::code_to_archive(code)
        };

        let cur_arch = self.runner_cache.get_or_create(contract_id, provide_arch)?;
        let actions = cur_arch.get_actions()?;

        let mut ctx = ApplyActionCtx {
            env: BTreeMap::new(),
            visited: BTreeSet::new(),
            contract_id,
        };
        match self.apply_action_recursive(vm, &mut ctx, &actions, contract_id)? {
            Some(e) => Ok(e),
            None => Err(anyhow::anyhow!(
                "actions returned by runner do not have a start instruction"
            )),
        }
    }
}
