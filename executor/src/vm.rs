use core::str;
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    io::{Read, Write},
    str::FromStr,
    sync::atomic::AtomicU32,
};

use genvm_modules_common::interfaces::{llm_functions_api, web_functions_api};
use itertools::Itertools;
use once_cell::sync::Lazy;
use wasmparser::WasmFeatures;
use wasmtime::{Engine, Linker, Module, Store};
use zip::ZipArchive;

use crate::{
    caching,
    runner::{self, InitAction, InitActionTrivial, WasmMode},
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
        WasmContext {
            genlayer_ctx: Arc::new(Mutex::new(wasi::Context::new(data, shared_data))),
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
    pub should_exit: Arc<AtomicU32>,
    // rust doesn't have aliasing Arc constructor
    //pub fuel_descriptor: Arc<wasmtime::FuelDescriptor>,
}

impl SharedData {
    fn new() -> Self {
        Self {
            nondet_call_no: 0.into(),
            should_exit: Arc::from(AtomicU32::from(0)),
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

#[derive(Clone)]
pub struct ContractCodeData {
    pub code: Arc<[u8]>,
    pub(crate) actions: Arc<InitAction>,
}

pub struct VM {
    pub store: Store<WasmContext>,
    pub linker: Arc<Mutex<Linker<WasmContext>>>,
    pub config_copy: wasi::base::Config,
    pub init_actions: ContractCodeData,
}

struct ApplyActionCtx {
    env: BTreeMap<String, String>,
    visited: BTreeSet<Arc<str>>,
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
        log::info!(target: "rt", event = "execution start"; "");
        let time_start = std::time::Instant::now();
        let res = func.call(&mut self.store, ());
        log::info!(target: "rt", event = "execution finished", duration:? = time_start.elapsed(); "");
        let res: RunResult = match res {
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
    pub runner_id: Arc<str>,
    pub debug_path: String,
    pub path_in_arch: Option<String>,
}

impl WasmFileDesc {
    pub fn special(&self) -> bool {
        return self.runner_id.is_empty() || self.runner_id.starts_with("<");
    }
}

impl Supervisor {
    pub fn new(modules: Modules, mut host: crate::Host) -> Result<Self> {
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
        let shared_data = Arc::new(SharedData::new());
        Ok(Self {
            engines,
            cached_modules: HashMap::new(),
            runner_cache: runner::RunnerReaderCache::new(),
            modules,
            host,
            shared_data,
        })
    }

    pub fn cache_module(&mut self, data: &WasmFileDesc) -> Result<Arc<PrecompiledModule>> {
        let entry = self.cached_modules.entry(data.contents.clone());
        match entry {
            std::collections::hash_map::Entry::Occupied(entry) => {
                log::debug!(target: "cache", cache_method = "rt", path = data.debug_path; "using rt cached");
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

                let debug_path = std::path::PathBuf::from_str(&data.debug_path)?;

                let compile_here = || -> Result<PrecompiledModule> {
                    log::info!(target: "cache", cache_method = "compiling", status = "start", path = data.debug_path; "");

                    let start_time = std::time::Instant::now();
                    let module_det = wasmtime::CodeBuilder::new(&self.engines.det)
                        .wasm_binary(&data.contents[..], Some(&debug_path))?
                        .compile_module()?;

                    let module_non_det = wasmtime::CodeBuilder::new(&self.engines.non_det)
                        .wasm_binary(&data.contents[..], Some(&debug_path))?
                        .compile_module()?;
                    log::info!(target: "cache", cache_method = "compiling", status = "done", duration:? = start_time.elapsed(), path = data.debug_path; "");
                    Ok(PrecompiledModule {
                        det: module_det,
                        non_det: module_non_det,
                    })
                };

                let get_from_runner = || -> Result<PrecompiledModule> {
                    if data.special() {
                        anyhow::bail!("special runners are not supported");
                    }
                    let path_in_arch = match &data.path_in_arch {
                        None => anyhow::bail!("no path in arch"),
                        Some(p) => p,
                    };
                    let mut result_zip_path = match Lazy::force(&caching::PRECOMPILE_DIR) {
                        Some(v) => v,
                        None => anyhow::bail!("cache is absent"),
                    }
                    .clone();

                    let (id, hash) = data
                        .runner_id
                        .split(":")
                        .collect_tuple()
                        .ok_or(anyhow::anyhow!("invalid runner id"))?;
                    let hash = format!("{hash}.zip");

                    result_zip_path.push(id);
                    result_zip_path.push(&hash);
                    let mut zip = ZipArchive::new(std::fs::File::open(&result_zip_path)?)?;

                    let mut process_single = |suff: &str, engine: &Engine| -> Result<Module> {
                        let mut buf = Vec::new();
                        let mut path_in_arch = path_in_arch.clone();
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

                    log::debug!(target: "cache", cache_method = "precompiled", path = data.debug_path; "using precompiled");

                    Ok(PrecompiledModule { det, non_det })
                };

                let ret = get_from_runner().or_else(|_e| compile_here())?;

                Ok(entry.insert(Arc::new(ret)).clone())
            }
        }
    }

    pub fn spawn(&mut self, data: crate::wasi::genlayer_sdk::SingleVMData) -> Result<VM> {
        let config_copy = data.conf.clone();
        let init_actions = data.init_actions.clone();

        let engine = if data.conf.is_deterministic {
            &self.engines.det
        } else {
            &self.engines.non_det
        };

        let store = Store::new(
            &engine,
            WasmContext::new(data, self.shared_data.clone()),
            self.shared_data.should_exit.clone(),
        );

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
            init_actions,
        })
    }

    fn link_wasm_into(&mut self, ret_vm: &mut VM, data: &WasmFileDesc) -> Result<wasmtime::Module> {
        let precompiled = self
            .cache_module(data)
            .with_context(|| format!("caching {:?}", &data.debug_path))?;
        if ret_vm.is_det() {
            Ok(precompiled.det.clone())
        } else {
            Ok(precompiled.non_det.clone())
        }
    }

    fn apply_single_action(
        &mut self,
        vm: &mut VM,
        ctx: &mut ApplyActionCtx,
        action: &InitAction,
    ) -> Result<Option<wasmtime::Instance>> {
        match action {
            InitAction::Trivial(InitActionTrivial::MapFile { to, contents }) => {
                vm.store
                    .data_mut()
                    .genlayer_ctx_mut()
                    .preview1
                    .map_file(&to, contents.clone())?;
                Ok(None)
            }
            InitAction::Trivial(InitActionTrivial::MapCode { to }) => {
                vm.store
                    .data_mut()
                    .genlayer_ctx_mut()
                    .preview1
                    .map_file(&to, vm.init_actions.code.clone())?;
                Ok(None)
            }
            InitAction::Trivial(InitActionTrivial::AddEnv { name, val }) => {
                let new_val = string_templater::patch_str(&ctx.env, val)?;
                ctx.env.insert(name.clone(), new_val);
                Ok(None)
            }
            InitAction::Trivial(InitActionTrivial::SetArgs(args)) => {
                vm.store
                    .data_mut()
                    .genlayer_ctx_mut()
                    .preview1
                    .set_args(&args[..])?;
                Ok(None)
            }
            InitAction::Trivial(InitActionTrivial::LinkWasm(data)) => {
                let module = self.link_wasm_into(vm, data)?;
                let instance = {
                    let Ok(ref mut linker) = vm.linker.lock() else {
                        panic!();
                    };
                    let instance = linker.instantiate(&mut vm.store, &module)?;
                    let name = module.name().ok_or(anyhow::anyhow!(
                        "can't link unnamed module {:?}",
                        &data.debug_path
                    ))?;
                    linker.instance(&mut vm.store, name, instance)?;
                    instance
                };
                match instance.get_typed_func::<(), ()>(&mut vm.store, "_initialize") {
                    Err(_) => {}
                    Ok(func) => {
                        log::info!(target: "rt", method = "call_initialize", wasm = data.debug_path; "");
                        func.call(&mut vm.store, ())?;
                    }
                }
                Ok(None)
            }
            InitAction::Trivial(InitActionTrivial::StartWasm(data)) => {
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
                let module = self.link_wasm_into(vm, data)?;
                let Ok(ref mut linker) = vm.linker.lock() else {
                    panic!();
                };
                Ok(Some(linker.instantiate(&mut vm.store, &module)?))
            }
            InitAction::When { cond, act } => {
                if (*cond == WasmMode::Det) != vm.is_det() {
                    return Ok(None);
                }
                self.apply_single_action(vm, ctx, act)
            }
            InitAction::Seq(vec) => {
                for act in vec {
                    match self.apply_single_action(vm, ctx, act)? {
                        Some(x) => return Ok(Some(x)),
                        None => {}
                    }
                }
                Ok(None)
            }
            InitAction::Once(id, action) => {
                if ctx.visited.insert(id.clone()) {
                    self.apply_single_action(vm, ctx, action)
                } else {
                    Ok(None)
                }
            }
        }
    }

    pub fn apply_actions(&mut self, vm: &mut VM) -> Result<wasmtime::Instance> {
        let mut ctx = ApplyActionCtx {
            env: BTreeMap::new(),
            visited: BTreeSet::new(),
        };
        match self.apply_single_action(vm, &mut ctx, &vm.init_actions.actions.clone())? {
            Some(e) => Ok(e),
            None => Err(anyhow::anyhow!(
                "actions returned by runner do not have a start instruction"
            )),
        }
    }

    pub fn get_actions_for(
        &mut self,
        contract_account: &crate::AccountAddress,
    ) -> Result<ContractCodeData> {
        let code = self.host.get_code(contract_account)?;
        let mut runner = runner::RunnerReader::new()?;
        let actions = if wasmparser::Parser::is_core_wasm(&code[..]) {
            InitAction::Trivial(InitActionTrivial::StartWasm(WasmFileDesc {
                contents: code.clone(),
                runner_id: Arc::from("<contract>"),
                debug_path: "<contract>".into(),
                path_in_arch: None,
            }))
        } else if let Ok(mut as_contr) = zip::ZipArchive::new(std::io::Cursor::new(&code)) {
            runner.get_for_archive(
                &Arc::from("<contract>"),
                &mut as_contr,
                &mut self.runner_cache,
            )?
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
            runner.get_for_archive(&Arc::from("<contract>"), &mut zip, &mut self.runner_cache)?
        };

        Ok(ContractCodeData {
            code,
            actions: Arc::new(actions),
        })
    }
}
