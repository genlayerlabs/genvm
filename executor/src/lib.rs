pub mod caching;
pub mod config;
pub mod host;
pub mod modules;
pub mod rt;
pub mod runners;
pub mod wasi;

pub use genlayer_sdk::abi::consts as public_abi;

pub use genvm_common::calldata;
use genvm_common::*;

pub use host::{Host, SlotID};

use anyhow::{Context, Result};
use wasi::genlayer_sdk::ExtendedMessage;

use std::sync::Arc;

use crate::wasi::genlayer_sdk::VMDataAccumulator;

pub fn wasmtime_to_anyhow(e: wasmtime::Error) -> anyhow::Error {
    match e.downcast() {
        Ok(anyhow_e) => anyhow_e,
        Err(e) => e.into(),
    }
}

pub fn anyhow_to_wasmtime(e: anyhow::Error) -> wasmtime::Error {
    match e.downcast() {
        Ok(wasmtime_e) => wasmtime_e,
        Err(e) => wasmtime::Error::from_anyhow(e),
    }
}

#[derive(Default, Debug, serde::Serialize)]
pub struct Metrics {
    pub supervisor: rt::Metrics,
    pub web_module: modules::Metrics,
    pub llm_module: modules::Metrics,
    pub hosts: Box<[host::Metrics]>,
}

impl<W: calldata::Writer> calldata::codec::Encode<W> for Metrics {
    type Error = W::Error;

    fn encode(&self, enc: &mut calldata::Encoder<W>) -> Result<(), Self::Error> {
        enc.start_map(4)?;
        enc.push_map_k("hosts")?;
        enc.start_array(self.hosts.len() as u64)?;
        for h in self.hosts.iter() {
            calldata::codec::Encode::encode(h, enc)?;
        }
        enc.push_map_k("llm_module")?;
        calldata::codec::Encode::encode(&self.llm_module, enc)?;
        enc.push_map_k("supervisor")?;
        calldata::codec::Encode::encode(&self.supervisor, enc)?;
        enc.push_map_k("web_module")?;
        calldata::codec::Encode::encode(&self.web_module, enc)?;
        Ok(())
    }
}

pub struct CreateSupervisorNamedArgs {
    pub method_hosts: Vec<u8>,
    pub gas_data: std::collections::BTreeMap<String, String>,
    pub initial_time_units_allocation: u32,
    pub leader_nondet_results: Option<Vec<bytes::Bytes>>,
}

pub fn create_supervisor(
    config: &config::Config,
    mut hosts: Vec<Host>,
    named: CreateSupervisorNamedArgs,
    host_data: genvm_modules_interfaces::HostData,
    shared_data: sync::DArc<rt::SharedData>,
    message: &domain::MessageData,
) -> Result<Arc<rt::supervisor::Supervisor>> {
    let metrics = shared_data.gep(|x| &x.metrics);

    let role = if named.leader_nondet_results.is_none() {
        genvm_modules_interfaces::Role::Leader
    } else {
        genvm_modules_interfaces::Role::Validator
    };

    let modules = modules::All {
        web: Arc::new(modules::Module::new(
            modules::ModuleNamedArgs {
                name: "web".into(),
                url: config.modules.web.address.clone(),
                gas_data: named.gas_data.clone(),
                initial_time_units_allocation: named.initial_time_units_allocation,
            },
            shared_data.cancellation.clone(),
            shared_data.genvm_id,
            role,
            host_data.clone(),
            metrics.gep(|x| &x.web_module),
        )),
        llm: Arc::new(modules::Module::new(
            modules::ModuleNamedArgs {
                name: "llm".into(),
                url: config.modules.llm.address.clone(),
                gas_data: named.gas_data,
                initial_time_units_allocation: named.initial_time_units_allocation,
            },
            shared_data.cancellation.clone(),
            shared_data.genvm_id,
            role,
            host_data,
            metrics.gep(|x| &x.llm_module),
        )),
    };

    let limiter_det = rt::memlimiter::Limiter::new("det");

    let storage_host_idx =
        if (host::host_fns::Methods::StorageRead as usize) < named.method_hosts.len() {
            named.method_hosts[host::host_fns::Methods::StorageRead as usize] as usize
        } else {
            0
        };

    let locked_slots = hosts[storage_host_idx]
        .get_locked_slots_for_sender(
            calldata::Address::from(message.contract_address.raw()),
            calldata::Address::from(message.sender_address.raw()),
            &limiter_det,
        )
        .context("reading locked slots")?;

    let multi_host = host::MultiHost::new(hosts, named.method_hosts);

    let ctor = rt::supervisor::Ctor {
        shared_data,
        modules,
        limiter: rt::DetNondet {
            det: limiter_det,
            non_det: rt::memlimiter::Limiter::new("nondet"),
        },
        locked_slots,
        leader_nondet_results: named.leader_nondet_results,
        multi_host,
    };

    rt::supervisor::Supervisor::start(config, ctor)
}

pub async fn run_with_impl(
    entry_data: domain::ExecutionData,
    supervisor: Arc<rt::supervisor::Supervisor>,
    permissions: &str,
) -> anyhow::Result<rt::vm::FullResult> {
    let storage_pages_limit = supervisor.get_storage_limiter();

    let mut topmost_storage = rt::vm::storage::Storage::new(
        entry_data.message.contract_address,
        storage_pages_limit,
        wasi::genlayer_sdk::StorageHostHolder(
            supervisor.host.clone(),
            wasi::genlayer_sdk::ReadToken {
                mode: public_abi::StorageType::LatestNonFinal,
                account: entry_data.message.contract_address,
            },
        ),
    );

    if let Some(code) = &entry_data.code {
        log_debug!("using provided code for execution");

        topmost_storage.write_code(code).await?;
    } else {
        log_debug!("code is null");
    }

    let data_fees_limit = supervisor.shared_data.gep(|x| &x.data_fees_limit);

    let essential_data = wasi::genlayer_sdk::SingleVMData {
        depth: 0,
        conf: wasi::base::Config {
            needs_error_fingerprint: true,
            is_deterministic: true,
            can_read_storage: permissions.contains("r"),
            can_write_storage: permissions.contains("w"),
            can_send_messages: permissions.contains("s"),
            can_call_others: permissions.contains("c"),
            can_spawn_nondet: permissions.contains("n"),
            state_mode: crate::public_abi::StorageType::Default,
        },
        message_data: ExtendedMessage {
            message: entry_data.message,
            entry_kind: public_abi::EntryKind::Main,
            entry_data: entry_data.calldata,
            entry_stage_data: calldata::Value::Null,
        },
        supervisor: supervisor.clone(),
        should_capture_fp: Arc::new(std::sync::atomic::AtomicBool::new(true)),

        storage: topmost_storage,
        accumulator: VMDataAccumulator {
            data_fees_limit,
            messages_value_decremented: primitive_types::U256::zero(),
            emissions: Vec::new(),
            message_fee_allocation: entry_data.message_fee_allocation,
        },
    };

    let run_result = rt::spawn_apply_run(&supervisor, essential_data).await?;

    Ok(rt::vm::FullResult {
        kind: match &run_result.run_ok {
            rt::vm::RunOk::Return(_) => public_abi::ResultCode::Return,
            rt::vm::RunOk::UserError(_) => public_abi::ResultCode::UserError,
            rt::vm::RunOk::VMError(_, _) => public_abi::ResultCode::VmError,
        },
        data: match run_result.run_ok {
            rt::vm::RunOk::Return(buf) => calldata::decode(&buf)?,
            rt::vm::RunOk::UserError(val) => val,
            rt::vm::RunOk::VMError(msg, _) => calldata::Value::Str(msg.0.into()),
        },
        fingerprint: run_result.fingerprint,
        storage_changes: run_result.vm_data.storage.make_delta(),
        emissions: run_result.vm_data.accumulator.emissions,
    })
}

pub async fn run_with(
    entry_data: domain::ExecutionData,
    supervisor: Arc<rt::supervisor::Supervisor>,
    permissions: &str,
) -> anyhow::Result<host::FullResult> {
    let res = run_with_impl(entry_data, supervisor.clone(), permissions).await;

    log_debug!("deterministic execution done");

    let nondet_disagree_res = rt::supervisor::await_nondet_vms(&supervisor).await;

    log_debug!("non-deterministic execution done");

    let merged_result = match (res, nondet_disagree_res) {
        (Err(e_res), Err(e_nondet)) => {
            log_error!(error:ah = e_nondet; "non-deterministic execution failed");

            Err(e_res)
        }
        (Err(e_res), Ok(_)) => Err(e_res),
        (Ok(_), Err(e_nondet)) => Err(e_nondet),
        (Ok(res), Ok(c)) => Ok((res, c)),
    };

    let res = if supervisor.shared_data.cancellation.is_cancelled() {
        match merged_result {
            Ok(mut r) => {
                if r.0.kind == public_abi::ResultCode::VmError {
                    r.0.data = calldata::Value::Str(public_abi::VmError::timeout().0.into());
                }
                Ok(r)
            }
            Err(_e) => Ok((rt::vm::FullResult::timeout(), None)),
        }
    } else {
        merged_result
    };

    let res = res.inspect_err(|e| {
        log_error!(error:ah = &e; "internal error");
    });

    if let Ok((_, Some(disag))) = &res {
        let mut host = supervisor
            .host
            .lock_for(host::host_fns::Methods::NotifyNondetDisagreement)
            .await;
        host.notify_nondet_disagreement(*disag)
            .context("notify non-deterministic disagreement")?;
    }

    log_debug!("all executions done, collecting stats");

    let is_timeout = supervisor.shared_data.cancellation.is_cancelled();

    let web_metrics = if is_timeout {
        None
    } else {
        supervisor
            .modules
            .web
            .get_stats(genvm_modules_interfaces::web::Message::GetStats)
            .await
            .ok()
    };
    let llm_metrics = if is_timeout {
        None
    } else {
        supervisor
            .modules
            .llm
            .get_stats(genvm_modules_interfaces::llm::Message::GetStats)
            .await
            .ok()
    };

    #[derive(serde::Serialize)]
    struct AllMetrics<'a> {
        web: Option<calldata::Value>,
        llm: Option<calldata::Value>,
        gvm: &'a crate::Metrics,
    }

    impl<W: calldata::Writer> calldata::codec::Encode<W> for AllMetrics<'_> {
        type Error = W::Error;

        fn encode(&self, enc: &mut calldata::Encoder<W>) -> Result<(), Self::Error> {
            enc.start_map(3)?;

            enc.push_map_k("gvm")?;
            calldata::codec::Encode::encode(self.gvm, enc)?;

            enc.push_map_k("llm")?;
            calldata::codec::Encode::encode(&self.llm, enc)?;

            enc.push_map_k("web")?;
            calldata::codec::Encode::encode(&self.web, enc)?;

            Ok(())
        }
    }

    let all_metrics = AllMetrics {
        web: web_metrics,
        llm: llm_metrics,
        gvm: &supervisor.shared_data.metrics,
    };

    let all_metrics = calldata::to_value(&all_metrics);

    log_info!(metrics:serde = all_metrics; "metrics");

    log_debug!("sending final result");

    let data_fees_remaining = supervisor.shared_data.data_fees_limit.remaining().await;

    let res = match res {
        Ok((a, b)) => Ok(host::FullResult::new(
            a,
            supervisor.take_nondet_results().await,
            b,
            data_fees_remaining,
        )),
        Err(e) => Err(e),
    };

    {
        let mut host = supervisor
            .host
            .lock_for(host::host_fns::Methods::ConsumeResult)
            .await;
        host.consume_result(&res).context("consume result")?;
    }

    {
        let mut host = supervisor
            .host
            .lock_for(host::host_fns::Methods::NotifyFinished)
            .await;
        host.notify_finished().context("notify finished")?;
    }

    host::all_useful_work_done();

    res
}
