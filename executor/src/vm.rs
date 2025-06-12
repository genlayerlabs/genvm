use core::str;
use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet, HashMap},
    sync::atomic::AtomicU32,
};

use itertools::Itertools;
use serde::Serialize;
use wasmparser::WasmFeatures;
use wasmtime::{Engine, Linker, Module, Store};

use crate::{
    caching, calldata, config,
    errors::ContractError,
    host::LockedSlotsSet,
    memlimiter,
    runner::{self, InitAction, WasmMode},
    ustar::{Archive, SharedBytes},
    wasi::{self, preview1::I32Exit},
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
                if !(2..=6).contains(&l) {
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

#[derive(Serialize)]
pub enum RunOk {
    Return(Vec<u8>),
    UserError(String),
    VMError(String, #[serde(skip_serializing)] Option<anyhow::Error>),
}

pub type RunResult = Result<RunOk>;

impl RunOk {
    pub fn empty_return() -> Self {
        Self::Return([0].into())
    }

    pub fn as_bytes_iter(&self) -> impl Iterator<Item = u8> + '_ {
        use crate::host::ResultCode;
        match self {
            RunOk::Return(buf) => [ResultCode::Return as u8]
                .into_iter()
                .chain(buf.iter().cloned()),
            RunOk::UserError(buf) => [ResultCode::UserError as u8]
                .into_iter()
                .chain(buf.as_bytes().iter().cloned()),
            RunOk::VMError(buf, _) => [ResultCode::VmError as u8]
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
            Self::UserError(r) => f.debug_tuple("UserError").field(r).finish(),
            Self::VMError(r, _) => f.debug_tuple("VMError").field(r).finish(),
        }
    }
}

#[derive(Clone)]
pub struct WasmContext {
    genlayer_ctx: Arc<Mutex<wasi::Context>>,
    limits: memlimiter::Limiter,
}

impl wasmtime::ResourceLimiter for memlimiter::Limiter {
    fn memory_growing(
        &mut self,
        current: usize,
        desired: usize,
        _maximum: Option<usize>,
    ) -> Result<bool> {
        let delta = desired - current;
        if delta > u32::MAX as usize {
            return Ok(false);
        }

        let delta = delta as u32;
        let success = self.consume(delta);

        if current == 0 && !success {
            Err(ContractError::oom(None).into())
        } else {
            Ok(success)
        }
    }

    fn table_growing(
        &mut self,
        current: usize,
        desired: usize,
        _maximum: Option<usize>,
    ) -> Result<bool> {
        let delta = desired - current;

        if delta > u32::MAX as usize {
            return Ok(false);
        }

        let delta = delta as u32;
        let success = self.consume_mul(delta, memlimiter::consts::TABLE_ENTRY_SIZE);

        if current == 0 && !success {
            Err(ContractError::oom(None).into())
        } else {
            Ok(success)
        }
    }

    fn instances(&self) -> usize {
        1000
    }

    fn tables(&self) -> usize {
        100
    }

    fn memories(&self) -> usize {
        100
    }
}

impl WasmContext {
    fn new(
        data: crate::wasi::genlayer_sdk::SingleVMData,
        shared_data: Arc<SharedData>,
        limiter: memlimiter::Limiter,
    ) -> anyhow::Result<WasmContext> {
        Ok(WasmContext {
            genlayer_ctx: Arc::new(Mutex::new(wasi::Context::new(data, shared_data)?)),
            limits: limiter,
        })
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

/// shared across all deterministic VMs
pub struct SharedData {
    pub nondet_call_no: AtomicU32,
    pub cancellation: Arc<genvm_common::cancellation::Token>,
    pub modules: Modules,
    pub balances: dashmap::DashMap<calldata::Address, primitive_types::U256>,
    pub is_sync: bool,
    pub cookie: String,
    pub allow_latest: bool,

    pub limiter_det: memlimiter::Limiter,
    pub limiter_non_det: memlimiter::Limiter,
    pub locked_slots: LockedSlotsSet,
}

impl SharedData {
    pub fn new(
        modules: Modules,
        cancellation: Arc<genvm_common::cancellation::Token>,
        is_sync: bool,
        cookie: String,
        allow_latest: bool,
        limiter_det: memlimiter::Limiter,
        locked_slots: LockedSlotsSet,
    ) -> Self {
        Self {
            nondet_call_no: 0.into(),
            cancellation,
            is_sync,
            modules,
            balances: dashmap::DashMap::new(),
            cookie,
            allow_latest,
            limiter_det,
            limiter_non_det: memlimiter::Limiter::new("non-det"),
            locked_slots,
        }
    }
}

pub struct PrecompiledModule {
    pub det: Module,
    pub non_det: Module,
}

pub struct Modules {
    pub web: Arc<crate::modules::Module>,
    pub llm: Arc<crate::modules::Module>,
}

#[derive(Serialize)]
struct SupervisorStats {
    precompile_hits: usize,
    cache_hits: usize,
    compiled_modules: usize,
}

pub struct Supervisor {
    pub host: crate::Host,
    pub shared_data: Arc<SharedData>,

    engines: Engines,
    cached_modules: HashMap<symbol_table::GlobalSymbol, Arc<PrecompiledModule>>,
    runner_cache: runner::RunnerReaderCache,
    cache_dir: Option<std::path::PathBuf>,

    stats: SupervisorStats,
}

pub struct VM {
    pub store: Store<WasmContext>,
    pub linker: Arc<tokio::sync::Mutex<Linker<WasmContext>>>,
    pub config_copy: wasi::base::Config,
}

struct ApplyActionCtx {
    env: BTreeMap<String, String>,
    visited: BTreeSet<symbol_table::GlobalSymbol>,
    contract_id: symbol_table::GlobalSymbol,
}

fn try_get_latest(runner_id: &str, base_path: &std::path::Path) -> Option<String> {
    let mut path = std::path::PathBuf::from(base_path);
    path.push("latest.json");

    let latest_registry = std::fs::read_to_string(&path).ok()?;
    let mut latest_registry: BTreeMap<String, String> =
        serde_json::from_str(&latest_registry).ok()?;

    latest_registry.remove(runner_id)
}

fn make_new_runner_arch_from_tar(
    id: symbol_table::GlobalSymbol,
    base_path: &std::path::Path,
    limiter: &memlimiter::Limiter,
) -> Result<Archive> {
    let (runner_id, runner_hash) =
        runner::verify_runner(id.as_str()).with_context(|| format!("verifying {id}"))?;

    let mut path = std::path::PathBuf::from(base_path);
    path.push(runner_id);

    let mut fname = runner_hash.to_owned();
    fname.push_str(".tar");
    path.push(fname);

    let contents = crate::mmap::load_file(&path, Some(limiter))?;

    crate::ustar::Archive::from_ustar(SharedBytes::new(contents))
        .with_context(|| format!("path {:?}", path))
}

impl VM {
    pub fn is_det(&self) -> bool {
        self.config_copy.is_deterministic
    }

    #[allow(clippy::manual_try_fold)]
    pub async fn run(&mut self, instance: &wasmtime::Instance) -> RunResult {
        if let Ok(lck) = self.store.data().genlayer_ctx.lock() {
            log::info!(wasi_preview1: serde = lck.preview1.log(), genlayer_sdk: serde = lck.genlayer_sdk.log(); "run");
        }

        let func = instance
            .get_typed_func::<(), ()>(&mut self.store, "")
            .or_else(|_| instance.get_typed_func::<(), ()>(&mut self.store, "_start"))
            .with_context(|| "can't find entrypoint")?;
        log::info!("execution start");
        let time_start = std::time::Instant::now();
        let res = func.call_async(&mut self.store, ()).await;
        log::info!(duration:? = time_start.elapsed(); "vm execution finished");
        let res: RunResult = match res {
            Ok(()) => Ok(RunOk::empty_return()),
            Err(e) => {
                let res: Result<RunOk> = [
                    |e: anyhow::Error| match e.downcast::<crate::wasi::preview1::I32Exit>() {
                        Ok(I32Exit(0)) => Ok(RunOk::empty_return()),
                        Ok(I32Exit(v)) => Ok(RunOk::VMError(format!("exit_code {}", v), None)),
                        Err(e) => Err(e),
                    },
                    |e: anyhow::Error| {
                        e.downcast::<wasmtime::Trap>()
                            .map(|v| RunOk::VMError(format!("wasm_trap {v:?}"), Some(v.into())))
                    },
                    |e: anyhow::Error| {
                        e.downcast::<crate::errors::ContractError>()
                            .map(|crate::errors::ContractError(m, c)| RunOk::VMError(m, c))
                    },
                    |e: anyhow::Error| {
                        e.downcast::<crate::errors::UserError>()
                            .map(|crate::errors::UserError(v)| RunOk::UserError(v))
                    },
                    |e: anyhow::Error| {
                        e.downcast::<crate::wasi::genlayer_sdk::ContractReturn>()
                            .map(|crate::wasi::genlayer_sdk::ContractReturn(v)| RunOk::Return(v))
                    },
                ]
                .into_iter()
                .fold(Err(e), |acc, func| match acc {
                    Ok(acc) => Ok(acc),
                    Err(e) => func(e),
                });
                res
            }
        };
        match &res {
            Ok(RunOk::Return(_)) => {
                log::info!(target: "vm", result = "Return"; "execution result unwrapped")
            }
            Ok(RunOk::UserError(_)) => {
                log::info!(target: "vm", result = "Rollback"; "execution result unwrapped")
            }
            Ok(RunOk::VMError(e, cause)) => {
                log::info!(target: "vm", result = format!("ContractError({e})"), cause:? = cause; "execution result unwrapped")
            }
            Err(_) => {
                log::info!(target: "vm", result = "Error"; "execution result unwrapped")
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
        base_conf.async_support(true);
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
        det_conf.wasm_floats_enabled(false);
        det_conf.cranelift_nan_canonicalization(true);

        let mut non_det_conf = base_conf.clone();
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
    pub contents: SharedBytes,
    pub runner_id: symbol_table::GlobalSymbol,
    pub path_in_arch: Arc<str>,
    pub wasm_uid: symbol_table::GlobalSymbol,
}

impl WasmFileDesc {
    pub fn new(
        contents: SharedBytes,
        runner_id: symbol_table::GlobalSymbol,
        path_in_arch: Arc<str>,
    ) -> Self {
        let mut wasm_uid = String::from(runner_id.as_str());
        wasm_uid.push(':');
        wasm_uid.push_str(&path_in_arch);

        Self {
            contents,
            runner_id,
            path_in_arch,
            wasm_uid: symbol_table::GlobalSymbol::from(wasm_uid),
        }
    }

    pub fn debug_path(&self) -> &'static str {
        self.wasm_uid.as_str()
    }

    pub fn is_special(&self) -> bool {
        self.runner_id.as_str().starts_with("<")
    }
}

impl Supervisor {
    #[allow(clippy::unnecessary_literal_unwrap)]
    pub fn new(
        config: &config::Config,
        mut host: crate::Host,
        shared_data: Arc<SharedData>,
    ) -> Result<Self> {
        let my_cache_dir = caching::get_cache_dir(&config.cache_dir).ok();

        let engines = Engines::create(|base_conf| {
            match &my_cache_dir {
                None => {
                    base_conf.disable_cache();
                }
                Some(cache_dir) => {
                    let mut cache_dir = cache_dir.to_owned();
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
        Ok(Self {
            engines,
            cached_modules: HashMap::new(),
            runner_cache: runner::RunnerReaderCache::new()?,
            host,
            shared_data,
            cache_dir: my_cache_dir,

            stats: SupervisorStats {
                cache_hits: 0,
                precompile_hits: 0,
                compiled_modules: 0,
            },
        })
    }

    pub fn cache_module(&mut self, data: &WasmFileDesc) -> Result<Arc<PrecompiledModule>> {
        let entry = self.cached_modules.entry(data.wasm_uid);
        match entry {
            std::collections::hash_map::Entry::Occupied(entry) => {
                log::debug!(target: "cache", cache_method = "rt", path = data.debug_path(); "using cached");
                self.stats.cache_hits += 1;
                Ok(entry.get().clone())
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                let debug_path = data.debug_path();

                let compile_here = || -> Result<PrecompiledModule> {
                    log::info!(status = "start", path = debug_path, runner = data.runner_id.as_str(); "cache compiling");

                    caching::validate_wasm(&self.engines, data.contents.as_ref())?;

                    let start_time = std::time::Instant::now();
                    let module_det = wasmtime::CodeBuilder::new(&self.engines.det)
                        .wasm_binary(
                            Cow::Borrowed(data.contents.as_ref()),
                            Some(std::path::Path::new(&debug_path)),
                        )?
                        .compile_module()?;

                    let module_non_det = wasmtime::CodeBuilder::new(&self.engines.non_det)
                        .wasm_binary(
                            Cow::Borrowed(data.contents.as_ref()),
                            Some(std::path::Path::new(&debug_path)),
                        )?
                        .compile_module()?;
                    log::info!(status = "done", duration:? = start_time.elapsed(), path = debug_path, runner = data.runner_id.as_str(); "cache compiling");
                    Ok(PrecompiledModule {
                        det: module_det,
                        non_det: module_non_det,
                    })
                };

                let get_from_precompiled = || -> Result<PrecompiledModule> {
                    if data.is_special() {
                        anyhow::bail!("special runners are not supported");
                    }
                    let (id, hash) = runner::verify_runner(data.runner_id.as_str())?;

                    let path_in_arch = data.path_in_arch.as_ref();
                    let mut result_zip_path = self
                        .cache_dir
                        .clone()
                        .ok_or_else(|| anyhow::anyhow!("cache is absent"))?;

                    result_zip_path.push(caching::PRECOMPILE_DIR_NAME);
                    result_zip_path.push(id);
                    result_zip_path.push(hash);
                    result_zip_path.push(caching::path_in_zip_to_hash(path_in_arch));

                    let process_single = |suff: &str, engine: &Engine| -> Result<Module> {
                        let path = result_zip_path.with_extension(suff);
                        unsafe { Module::deserialize_file(engine, &path) }
                    };

                    let det = process_single(
                        caching::DET_NON_DET_PRECOMPILED_SUFFIX.det,
                        &self.engines.det,
                    )?;
                    let non_det = process_single(
                        caching::DET_NON_DET_PRECOMPILED_SUFFIX.non_det,
                        &self.engines.non_det,
                    )?;

                    log::debug!(target: "cache", cache_method = "precompiled", runner = data.runner_id.as_str(); "using cached");

                    Ok(PrecompiledModule { det, non_det })
                };

                let ret = get_from_precompiled().inspect(|_| { self.stats.precompile_hits += 1; }).or_else(|e| {
                    log::trace!(target: "cache", error:serde = genvm_common::LogError(&e), runner = data.runner_id.as_str(); "could not use precompiled");
                    self.stats.compiled_modules += 1;
                    compile_here()
                })?;

                Ok(entry.insert(Arc::new(ret)).clone())
            }
        }
    }

    pub async fn spawn(&mut self, data: crate::wasi::genlayer_sdk::SingleVMData) -> Result<VM> {
        let config_copy = data.conf;

        let (engine, limiter) = if data.conf.is_deterministic {
            (&self.engines.det, self.shared_data.limiter_det.clone())
        } else {
            (
                &self.engines.non_det,
                self.shared_data.limiter_non_det.clone(),
            )
        };

        let mut store = Store::new(
            engine,
            WasmContext::new(data, self.shared_data.clone(), limiter)?,
            self.shared_data.cancellation.should_quit.clone(),
        );

        store.limiter(|ctx| &mut ctx.limits);

        let linker_shared = Arc::new(tokio::sync::Mutex::new(Linker::new(engine)));

        {
            let mut linker = linker_shared.lock().await;
            linker.allow_unknown_exports(false);
            linker.allow_shadowing(false);

            crate::wasi::add_to_linker_sync(
                &mut linker,
                linker_shared.clone(),
                |host: &mut WasmContext| host.genlayer_ctx_mut(),
            )?;
        }

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

    fn unfold_test_id_if_any(
        &mut self,
        ctx: &ApplyActionCtx,
        id: symbol_table::GlobalSymbol,
        path: &std::path::Path,
    ) -> Result<symbol_table::GlobalSymbol> {
        if id.as_str() == "<contract>" {
            Ok(ctx.contract_id)
        } else {
            let (runner_id, runner_hash) =
                runner::verify_runner(id.as_str()).with_context(|| format!("verifying {id}"))?;

            if runner_hash == "test" || runner_hash == "latest" {
                if !self.shared_data.allow_latest {
                    anyhow::bail!("test/latest runner not allowed")
                }

                if let Some(borrowed) = try_get_latest(runner_id, path) {
                    let mut new_id = runner_id.to_owned();
                    new_id.push(':');
                    new_id.push_str(&borrowed);

                    return Ok(symbol_table::GlobalSymbol::new(new_id));
                }
            }

            Ok(id)
        }
    }

    async fn apply_action_recursive(
        &mut self,
        vm: &mut VM,
        ctx: &mut ApplyActionCtx,
        action: &InitAction,
        current: symbol_table::GlobalSymbol,
    ) -> Result<Option<wasmtime::Instance>> {
        match action {
            InitAction::MapFile { to, file } => {
                let limiter = if vm.is_det() {
                    &self.shared_data.limiter_det
                } else {
                    &self.shared_data.limiter_non_det
                };

                if file.ends_with("/") {
                    let arch = self.runner_cache.get_unsafe(current);

                    let file_name_str = String::from(&file[..]);

                    for (name, file_contents) in arch.files.data.range(file_name_str..) {
                        if name.ends_with("/") {
                            continue;
                        }

                        if !name.starts_with(&file[..]) {
                            break;
                        }

                        let mut name_in_fs = String::from(&to[..]);
                        if !name_in_fs.ends_with("/") {
                            name_in_fs.push('/');
                        }
                        name_in_fs.push_str(&name[file.len()..]);

                        if !limiter.consume(
                            memlimiter::consts::FILE_MAPPING_SIZE + name_in_fs.len() as u32,
                        ) {
                            return Err(ContractError::oom(None).into());
                        }

                        vm.store
                            .data_mut()
                            .genlayer_ctx_mut()
                            .preview1
                            .map_file(&name_in_fs, file_contents.clone())?;
                    }
                } else {
                    if !limiter.consume(memlimiter::consts::FILE_MAPPING_SIZE + to.len() as u32) {
                        return Err(ContractError::oom(None).into());
                    }

                    vm.store
                        .data_mut()
                        .genlayer_ctx_mut()
                        .preview1
                        .map_file(to, self.runner_cache.get_unsafe(current).get_file(file)?)?;
                }
                Ok(None)
            }
            InitAction::AddEnv { name, val } => {
                let new_val = genvm_common::templater::patch_str(
                    &ctx.env,
                    val,
                    &genvm_common::templater::DOLLAR_UNFOLDER_RE,
                )?;
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
                let contents = self.runner_cache.get_unsafe(current).get_file(path)?;

                let module =
                    self.link_wasm_into(vm, &WasmFileDesc::new(contents, current, path.clone()))?;
                let instance = {
                    let mut linker = vm.linker.lock().await;
                    let instance = linker.instantiate_async(&mut vm.store, &module).await?;
                    let name = module
                        .name()
                        .ok_or_else(|| anyhow::anyhow!("can't link unnamed module {:?}", current))
                        .map_err(|e| {
                            crate::errors::ContractError("invalid_wasm".into(), Some(e))
                        })?;
                    linker.instance(&mut vm.store, name, instance)?;
                    instance
                };
                match instance.get_typed_func::<(), ()>(&mut vm.store, "_initialize") {
                    Err(_) => {}
                    Ok(func) => {
                        log::info!(target: "rt", runner = self.runner_cache.get_unsafe(current).runner_id().as_str(), path = path; "calling _initialize");
                        func.call_async(&mut vm.store, ()).await?;
                    }
                }
                Ok(None)
            }
            InitAction::StartWasm(path) => {
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
                let module =
                    self.link_wasm_into(vm, &WasmFileDesc::new(contents, current, path.clone()))?;

                let linker = vm.linker.lock().await;
                Ok(Some(
                    linker.instantiate_async(&mut vm.store, &module).await?,
                ))
            }
            InitAction::When { cond, action } => {
                if (*cond == WasmMode::Det) != vm.is_det() {
                    return Ok(None);
                }
                Box::pin(self.apply_action_recursive(vm, ctx, action, current)).await
            }
            InitAction::Seq(vec) => {
                for act in vec {
                    if let Some(x) =
                        Box::pin(self.apply_action_recursive(vm, ctx, act, current)).await?
                    {
                        return Ok(Some(x));
                    }
                }
                Ok(None)
            }
            InitAction::With { runner: id, action } => {
                let path = self.runner_cache.path().clone();

                let id = self.unfold_test_id_if_any(ctx, *id, &path)?;

                let limiter = if vm.is_det() {
                    &self.shared_data.limiter_det
                } else {
                    &self.shared_data.limiter_non_det
                };

                let _ = self.runner_cache.get_or_create(
                    id,
                    || make_new_runner_arch_from_tar(id, &path, limiter),
                    limiter,
                )?;

                Box::pin(self.apply_action_recursive(vm, ctx, action, id))
                    .await
                    .with_context(|| format!("With {id}"))
            }
            InitAction::Depends(id) => {
                let path = self.runner_cache.path().clone();

                let id = self.unfold_test_id_if_any(ctx, *id, &path)?;

                if !ctx.visited.insert(id) {
                    return Ok(None);
                }

                let limiter = if vm.is_det() {
                    &self.shared_data.limiter_det
                } else {
                    &self.shared_data.limiter_non_det
                };

                let path = self.runner_cache.path().clone();
                let new_arch = self.runner_cache.get_or_create(
                    id,
                    || {
                        make_new_runner_arch_from_tar(id, &path, limiter)
                            .with_context(|| format!("loading {id}"))
                    },
                    limiter,
                )?;
                let new_action = new_arch
                    .get_actions()
                    .with_context(|| format!("loading {id} runner.json"))?;
                Box::pin(self.apply_action_recursive(vm, ctx, &new_action, id))
                    .await
                    .with_context(|| format!("Depends {id}"))
            }
        }
    }

    fn code_to_archive(code: SharedBytes) -> Result<Archive> {
        if let Ok(mut as_zip) = zip::ZipArchive::new(std::io::Cursor::new(code.clone())) {
            return Archive::from_zip(&mut as_zip, code.len() as u32);
        }

        if wasmparser::Parser::is_core_wasm(code.as_ref()) {
            return Ok(Archive::from_file_and_runner(
                code,
                SharedBytes::from(&b"{ \"StartWasm\": \"file\" }"[..]),
            ));
        }
        let code_str = str::from_utf8(code.as_ref()).map_err(|e| {
            crate::errors::ContractError(
                "invalid_contract non-utf8".into(),
                Some(anyhow::Error::from(e)),
            )
        })?;
        let code_start = (|| {
            for c in ["//", "#", "--"] {
                if code_str.starts_with(c) {
                    return Ok(c);
                }
            }
            Err(crate::errors::ContractError(
                "no_runner_comment".into(),
                None,
            ))
        })()?;
        let mut code_comment = String::new();
        for l in code_str.lines() {
            if !l.starts_with(code_start) {
                break;
            }
            code_comment.push_str(&l[code_start.len()..])
        }

        Ok(Archive::from_file_and_runner(
            code,
            SharedBytes::from(code_comment.as_bytes()),
        ))
    }

    pub async fn apply_contract_actions(&mut self, vm: &mut VM) -> Result<wasmtime::Instance> {
        let contract_address = {
            let lock = vm.store.data().genlayer_ctx.lock().unwrap();
            lock.genlayer_sdk.data.message_data.contract_address
        };

        let contract_id = runner::get_id_of_contract(contract_address);

        let limiter = if vm.is_det() {
            &self.shared_data.limiter_det
        } else {
            &self.shared_data.limiter_non_det
        };

        let provide_arch = || {
            let code = self
                .host
                .get_code(vm.config_copy.state_mode, contract_address, limiter)?;
            Self::code_to_archive(SharedBytes::new(code))
        };

        let cur_arch = self
            .runner_cache
            .get_or_create(contract_id, provide_arch, limiter)?;
        let actions = cur_arch.get_actions()?;

        let mut ctx = ApplyActionCtx {
            env: BTreeMap::new(),
            visited: BTreeSet::new(),
            contract_id,
        };
        match self
            .apply_action_recursive(vm, &mut ctx, &actions, contract_id)
            .await?
        {
            Some(e) => Ok(e),
            None => Err(anyhow::anyhow!(
                "actions returned by runner do not have a start instruction"
            )),
        }
    }

    pub fn log_stats(&self) {
        log::debug!(
            all_wasm_modules:serde = self.cached_modules.keys().map(|x| x.as_str()).collect_vec(),
            stats:serde = self.stats,
            det_max_memory = u32::MAX - self.shared_data.limiter_det.get_least_remaining_memory(),
            non_det_max_memory = u32::MAX - self.shared_data.limiter_non_det.get_least_remaining_memory();
            "supervisor stats"
        );
    }
}
