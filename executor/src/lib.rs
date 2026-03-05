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

use anyhow::Result;
use wasi::genlayer_sdk::ExtendedMessage;

use std::sync::Arc;

#[derive(Default, Debug, serde::Serialize)]
pub struct Metrics {
    pub supervisor: rt::Metrics,
    pub web_module: modules::Metrics,
    pub llm_module: modules::Metrics,
    pub hosts: Box<[host::Metrics]>,
}

pub fn create_supervisor(
    config: &config::Config,
    mut hosts: Vec<Host>,
    method_hosts: Vec<u8>,
    host_data: genvm_modules_interfaces::HostData,
    shared_data: sync::DArc<rt::SharedData>,
    message: &domain::MessageData,
    leader_nondet_results: Option<Vec<bytes::Bytes>>,
) -> Result<Arc<rt::supervisor::Supervisor>> {
    let metrics = shared_data.gep(|x| &x.metrics);

    let modules = modules::All {
        web: Arc::new(modules::Module::new(
            "web".into(),
            config.modules.web.address.clone(),
            shared_data.cancellation.clone(),
            shared_data.genvm_id,
            host_data.clone(),
            metrics.gep(|x| &x.web_module),
        )),
        llm: Arc::new(modules::Module::new(
            "llm".into(),
            config.modules.llm.address.clone(),
            shared_data.cancellation.clone(),
            shared_data.genvm_id,
            host_data,
            metrics.gep(|x| &x.llm_module),
        )),
    };

    let limiter_det = rt::memlimiter::Limiter::new("det");

    let storage_host_idx = if (host::host_fns::Methods::StorageRead as usize) < method_hosts.len() {
        method_hosts[host::host_fns::Methods::StorageRead as usize] as usize
    } else {
        0
    };

    let locked_slots = hosts[storage_host_idx].get_locked_slots_for_sender(
        calldata::Address::from(message.contract_address.raw()),
        calldata::Address::from(message.sender_address.raw()),
        &limiter_det,
    )?;

    let multi_host = host::MultiHost::new(hosts, method_hosts);

    let ctor = rt::supervisor::Ctor {
        shared_data,
        modules,
        limiter: rt::DetNondet {
            det: limiter_det,
            non_det: rt::memlimiter::Limiter::new("nondet"),
        },
        locked_slots,
        leader_nondet_results,
        multi_host,
    };

    rt::supervisor::Supervisor::start(config, ctor)
}

fn log_vm_error(e: &anyhow::Error) {
    if let Some(rt::errors::VMError(msg, Some(err))) = e.downcast_ref() {
        log_error!(msg = msg, error:ah = err; "vm error");
    } else {
        log_error!(error:ah = e; "vm error");
    }
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

    let essential_data = wasi::genlayer_sdk::SingleVMData {
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
        emissions: Vec::new(),
    };

    let limiter = supervisor
        .limiter
        .get(essential_data.conf.is_deterministic)
        .derived();

    let vm = rt::supervisor::spawn(&supervisor, essential_data, limiter)
        .await
        .inspect_err(log_vm_error)?;
    let vm = rt::supervisor::apply_contract_actions(&supervisor, vm)
        .await
        .inspect_err(log_vm_error);

    let vm = match vm {
        Err(e) => {
            return match rt::errors::unwrap_vm_errors(e) {
                Err(e) => Err(e),
                Ok(v) => Ok(rt::vm::FullResult::empty_from(v)),
            };
        }
        Ok(v) => v,
    };

    let run_result = vm.run().await?;

    Ok(rt::vm::FullResult {
        kind: match &run_result.run_ok {
            rt::vm::RunOk::Return(_) => public_abi::ResultCode::Return,
            rt::vm::RunOk::UserError(_) => public_abi::ResultCode::UserError,
            rt::vm::RunOk::VMError(_, _) => public_abi::ResultCode::VmError,
        },
        data: match run_result.run_ok {
            rt::vm::RunOk::Return(buf) => calldata::decode(&buf)?,
            rt::vm::RunOk::UserError(buf) => calldata::Value::Str(buf),
            rt::vm::RunOk::VMError(msg, _) => calldata::Value::Str(msg),
        },
        fingerprint: run_result.fingerprint,
        storage_changes: run_result.vm_data.storage.make_delta(),
        emissions: run_result.vm_data.emissions,
        //nondet_results: supervisor.take_nondet_results().await,
        //nondet_disagreement: supervisor.,
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
                    r.0.data = calldata::Value::Str(public_abi::VmError::Timeout.value().into());
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
        host.notify_nondet_disagreement(*disag)?;
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

    let all_metrics = AllMetrics {
        web: web_metrics,
        llm: llm_metrics,
        gvm: &supervisor.shared_data.metrics,
    };

    let all_metrics = calldata::to_value(&all_metrics)
        .ok()
        .unwrap_or(calldata::Value::Null);

    log_info!(metrics:serde = all_metrics; "metrics");

    log_debug!("sending final result");

    let res = match res {
        Ok((a, b)) => Ok(host::FullResult::new(
            a,
            supervisor.take_nondet_results().await,
            b,
        )),
        Err(e) => Err(e),
    };

    {
        let mut host = supervisor
            .host
            .lock_for(host::host_fns::Methods::ConsumeResult)
            .await;
        host.consume_result(&res)?;
    }

    {
        let mut host = supervisor
            .host
            .lock_for(host::host_fns::Methods::NotifyFinished)
            .await;
        host.notify_finished()?;
    }

    host::all_useful_work_done();

    res
}
