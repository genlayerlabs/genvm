use std::{
    collections::{BTreeMap, HashSet},
    sync::{atomic::AtomicU32, Arc},
};

use genvm_common::*;

use crate::{
    config, host, public_abi,
    rt::{self, DetNondet},
    runners, wasi,
};

mod actions;
mod compilation;

struct WasmModuleCache {
    cache_dir: Option<std::path::PathBuf>,
    wasm_modules_cache: sync::CacheMap<DetNondet<wasmtime::Module>>,
}

pub struct NonDetVMTask {
    pub task: wasi::genlayer_sdk::SingleVMData,
    pub call_no: u32,
}

impl NonDetVMTask {
    pub async fn run_now(self, sup: &Arc<Supervisor>) -> anyhow::Result<rt::vm::RunOk> {
        run_single_nondet(sup, self).await
    }
}

pub struct VMCountDecrementer(Arc<Supervisor>);

impl std::ops::Drop for VMCountDecrementer {
    fn drop(&mut self) {
        self.0.queue.vm_countdown.decrement();
    }
}

struct NondetQueue {
    sender: tokio::sync::mpsc::Sender<sync::Lock<NonDetVMTask, VMCountDecrementer>>,
    nondet_call_disagree: std::sync::atomic::AtomicU32,
    vm_countdown: genvm_common::sync::Waiter,
    tasks_loop_done: tokio::sync::Notify,
    encountered_error: crossbeam::atomic::AtomicCell<Option<anyhow::Error>>,
}

pub struct Ctor {
    pub shared_data: sync::DArc<rt::SharedData>,

    pub modules: crate::modules::All,

    pub limiter: rt::DetNondet<rt::memlimiter::Limiter>,
    pub locked_slots: host::LockedSlotsSet,
}

pub struct Supervisor {
    pub shared_data: sync::DArc<rt::SharedData>,
    pub modules: crate::modules::All,
    pub limiter: rt::DetNondet<rt::memlimiter::Limiter>,
    pub locked_slots: host::LockedSlotsSet,

    pub nondet_call_no: AtomicU32,
    pub balances: dashmap::DashMap<calldata::Address, primitive_types::U256>,

    queue: NondetQueue,
    runner_cache: runners::cache::Reader,
    wasm_mod_cache: WasmModuleCache,

    pub(crate) engines: rt::DetNondet<wasmtime::Engine>,
    pub(crate) host: tokio::sync::Mutex<host::Host>,
}

pub fn create_engines(
    config_base: impl FnOnce(&mut wasmtime::Config) -> anyhow::Result<()>,
) -> anyhow::Result<rt::DetNondet<wasmtime::Engine>> {
    let mut base_conf = wasmtime::Config::default();

    base_conf
        .debug_info(true)
        .wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Disable)
        .async_support(true)
        .consume_fuel(false)
        .cranelift_opt_level(wasmtime::OptLevel::None);

    base_conf
        .wasm_tail_call(true)
        .wasm_bulk_memory(true)
        .wasm_simd(false)
        .relaxed_simd_deterministic(true)
        .wasm_relaxed_simd(false);

    use wasmparser::WasmFeatures;

    base_conf
        .wasm_feature(WasmFeatures::BULK_MEMORY, true)
        .wasm_feature(WasmFeatures::SIGN_EXTENSION, true)
        .wasm_feature(WasmFeatures::MUTABLE_GLOBAL, true)
        .wasm_feature(WasmFeatures::MULTI_VALUE, true)
        .wasm_feature(WasmFeatures::SATURATING_FLOAT_TO_INT, false)
        .wasm_feature(WasmFeatures::REFERENCE_TYPES, false);

    config_base(&mut base_conf)?;

    let mut det_conf = base_conf.clone();
    det_conf
        .wasm_floats_enabled(false)
        .cranelift_nan_canonicalization(true)
        .wasm_backtrace(true);

    let mut non_det_conf = base_conf.clone();
    non_det_conf.wasm_floats_enabled(true).wasm_backtrace(false);

    let det_engine = wasmtime::Engine::new(&det_conf)?;
    let non_det_engine = wasmtime::Engine::new(&non_det_conf)?;

    Ok(rt::DetNondet {
        det: det_engine,
        non_det: non_det_engine,
    })
}

pub async fn await_nondet_vms(zelf: &Arc<Supervisor>) -> anyhow::Result<Option<u32>> {
    zelf.queue.vm_countdown.decrement();

    zelf.queue.tasks_loop_done.notified().await;
    if let Some(err) = zelf.queue.encountered_error.take() {
        return Err(err);
    }

    let disagree_call = zelf
        .queue
        .nondet_call_disagree
        .load(std::sync::atomic::Ordering::SeqCst);
    if disagree_call == u32::MAX {
        return Ok(None);
    }

    Ok(Some(disagree_call))
}

pub async fn submit_nondet_vm_task(zelf: &Arc<Supervisor>, task: NonDetVMTask) {
    let call_no = task.call_no;

    zelf.queue.vm_countdown.increment();
    let tok = VMCountDecrementer(zelf.clone());
    let permit = zelf.queue.sender.reserve().await.unwrap();
    permit.send(sync::Lock::new(task, tok));

    log_debug!(call_no = call_no; "nondet vm task submitted");
}

impl Supervisor {
    pub fn start(
        config: &config::Config,
        ctor: Ctor,
        host: host::Host,
    ) -> anyhow::Result<Arc<Self>> {
        let my_cache_dir = runners::cache::get_cache_dir(&config.cache_dir).ok();

        let engines = create_engines(|base_conf| {
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
        })?;

        let (sender, receiver) = tokio::sync::mpsc::channel(100);

        let debug_mode = ctor.shared_data.debug_mode;

        let zelf = Arc::new(Self {
            shared_data: ctor.shared_data,
            modules: ctor.modules,
            limiter: ctor.limiter,
            locked_slots: ctor.locked_slots,
            nondet_call_no: AtomicU32::new(0),
            balances: dashmap::DashMap::new(),
            queue: NondetQueue {
                sender,
                encountered_error: crossbeam::atomic::AtomicCell::new(None),
                nondet_call_disagree: std::sync::atomic::AtomicU32::new(u32::MAX),
                vm_countdown: genvm_common::sync::Waiter::new(),
                tasks_loop_done: tokio::sync::Notify::new(),
            },
            runner_cache: runners::cache::Reader::new(
                std::path::Path::new(&config.runners_dir),
                std::path::Path::new(&config.registry_dir),
                debug_mode,
            )?,
            wasm_mod_cache: WasmModuleCache {
                cache_dir: my_cache_dir,
                wasm_modules_cache: sync::CacheMap::new(),
            },
            host: tokio::sync::Mutex::new(host),
            engines,
        });

        tokio::spawn(nondet_vm_processor(zelf.clone(), receiver));

        Ok(zelf)
    }
}

pub async fn spawn(
    zelf: &Arc<Supervisor>,
    vm: wasi::genlayer_sdk::SingleVMData,
    limiter: rt::memlimiter::Limiter,
) -> anyhow::Result<rt::vm::VM<()>> {
    let config_copy = vm.conf;

    let engine = zelf.engines.get(vm.conf.is_deterministic);

    let should_capture_fp = std::sync::Arc::new(vm.conf.is_deterministic.into());

    let mut store = wasmtime::Store::new(
        engine,
        rt::vm::WasmtimeStoreData {
            limits: limiter.clone(),
            genlayer_ctx: std::sync::Arc::new(std::sync::Mutex::new(wasi::Context::new(
                vm, limiter,
            )?)),
            supervisor: zelf.clone(),
        },
        wasmtime::GenVMCtx {
            should_capture_fp,
            should_quit: zelf.shared_data.cancellation.should_quit.clone(),
        },
    );

    store.limiter(|ctx| &mut ctx.limits);

    let mut linker = wasmtime::Linker::new(engine);

    linker.allow_unknown_exports(false);
    linker.allow_shadowing(false);

    wasi::add_to_linker_sync(&mut linker, |host: &mut rt::vm::WasmtimeStoreData| {
        host.genlayer_ctx_mut()
    })?;

    Ok(rt::vm::VM {
        vm_base: rt::vm::VMBase {
            store,
            linker,
            config_copy,
        },
        data: (),
    })
}

pub async fn apply_contract_actions(
    zelf: &std::sync::Arc<Supervisor>,
    mut vm: rt::vm::VM<()>,
) -> anyhow::Result<rt::vm::VM<wasmtime::Instance>> {
    let (contract_address, datetime) = {
        let lock = vm.vm_base.store.data().genlayer_ctx.lock().unwrap();
        (
            lock.genlayer_sdk.data.message_data.contract_address,
            lock.genlayer_sdk.data.message_data.datetime,
        )
    };

    let contract_id = runners::get_runner_of_contract(contract_address);

    let limiter = vm.vm_base.store.data_mut().limits.clone();

    let arch = zelf
        .runner_cache
        .get_or_create(
            contract_id,
            || async {
                let mut host = zelf.host.lock().await;

                let code = host.get_code(
                    vm.vm_base.config_copy.state_mode,
                    contract_address,
                    &limiter,
                )?;

                std::mem::drop(host);

                runners::parse(util::SharedBytes::new(code))
            },
            &limiter,
        )
        .await
        .map_err(|e| {
            rt::errors::VMError::wrap(public_abi::VmError::InvalidContract.value().to_owned(), e)
        })?;

    let version = arch.get_version().map_err(|e| {
        rt::errors::VMError::wrap(public_abi::VmError::InvalidContract.value().to_owned(), e)
    })?;

    //self.shared_data.
    let transaction_ts = datetime.timestamp() as u64;
    let max_version_index =
        match crate::version_timestamps::DATA.binary_search_by_key(&transaction_ts, |x| x.0) {
            Ok(index) => index,
            Err(0) => 0,
            Err(index) => index - 1,
        };

    if version > crate::version_timestamps::DATA[max_version_index].1 {
        log_info!(
            version = version,
            max_version = crate::version_timestamps::DATA[max_version_index].1,
            requested_timestamp = transaction_ts;
            "version is too big, rejecting"
        );
        return Err(
            rt::errors::VMError(public_abi::VmError::VersionTooBig.value().into(), None).into(),
        );
    }

    vm.vm_base
        .store
        .data_mut()
        .genlayer_ctx_mut()
        .genlayer_sdk
        .data
        .version = version;

    let actions = arch.get_actions().await.map_err(|e| {
        rt::errors::VMError::wrap(public_abi::VmError::InvalidContract.value().to_owned(), e)
    })?;

    let mut ctx = actions::Ctx {
        env: BTreeMap::new(),
        visited: HashSet::new(),
        contract_id,
        supervisor: zelf,
        vm: &mut vm.vm_base,
    };

    let inst = match ctx.apply(&actions, contract_id, &arch).await {
        Ok(Some(inst)) => inst,
        Ok(None) => {
            return Err(anyhow::anyhow!(
                "actions returned by runner do not have a start instruction"
            ));
        }
        Err(e) => {
            return Err(rt::errors::VMError::wrap(
                public_abi::VmError::InvalidContract.value().into(),
                e,
            )
            .into());
        }
    };

    Ok(rt::vm::VM {
        vm_base: vm.vm_base,
        data: inst,
    })
}

async fn run_single_nondet(
    zelf: &std::sync::Arc<Supervisor>,
    task: NonDetVMTask,
) -> anyhow::Result<rt::vm::RunOk> {
    let limiter = zelf.limiter.get(false).derived();
    let vm = spawn(zelf, task.task, limiter).await?;
    let vm = apply_contract_actions(zelf, vm).await?;
    vm.run().await.map(|x| x.0)
}

async fn nondet_vm_processor(
    zelf: std::sync::Arc<Supervisor>,
    mut nondet_validator_queue_receiver: tokio::sync::mpsc::Receiver<
        sync::Lock<NonDetVMTask, VMCountDecrementer>,
    >,
) {
    let mut count = 0;
    loop {
        tokio::select! {
            _ = zelf.shared_data.cancellation.chan.closed() => {
                log_debug!("cancellation requested, stopping nondet validator queue");
                break;
            }

            _ = zelf.queue.vm_countdown.wait() => {
                log_debug!("vm countdown reached zero, stopping nondet validator queue");
                break;
            }

            Some(task) = nondet_validator_queue_receiver.recv() => {
                count += 1;
                if zelf.queue.nondet_call_disagree.load(std::sync::atomic::Ordering::SeqCst) != u32::MAX {
                    log_info!("skipped nondet block due to disagreement in previous one");
                    continue;
                }

                let call_no = task.call_no;

                let (task, tok) = task.deconstruct();
                let res = run_single_nondet(&zelf, task).await;

                let do_disagree = match res {
                    Ok(rt::vm::RunOk::Return(v)) if v == [16] => false,
                    Ok(rt::vm::RunOk::Return(v)) if v == [8] => true,
                    Ok(other) => {
                        log_warn!(result:? = other; "unexpected result in nondet block, setting to disagree");
                        true
                    }
                    Err(e) => {
                        if let Some(old_err) = zelf.queue.encountered_error.swap(Some(e)) {
                            log_error!(error:ah = old_err; "encountered another error, overwriting");
                        }
                        continue;
                    }
                };

                log_trace!(call_no = call_no, do_disagree = do_disagree; "nondet call result");

                if do_disagree {
                    zelf.queue.nondet_call_disagree
                        .fetch_min(call_no, std::sync::atomic::Ordering::SeqCst);
                }

                std::mem::drop(tok);
            }
        }
    }

    log_debug!(count = count; "all nondet vms done");

    zelf.queue.tasks_loop_done.notify_one();
}
