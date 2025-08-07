pub mod caching;
pub mod config;
mod host;
pub mod modules;
pub mod rt;
pub mod runners;
pub mod wasi;

pub mod public_abi;

pub mod version_timestamps;

pub use genvm_common::calldata;
use genvm_common::*;

pub use host::{Host, MessageData, SlotID};

use anyhow::Result;
use wasi::genlayer_sdk::ExtendedMessage;

use std::{str::FromStr, sync::Arc};

#[derive(Default, Debug, serde::Serialize)]
pub struct Metrics {
    pub supervisor: rt::Metrics,
    pub host: host::Metrics,
    pub web_module: modules::Metrics,
    pub llm_module: modules::Metrics,
}

pub fn create_supervisor(
    config: &config::Config,
    mut host: Host,
    host_data: genvm_modules_interfaces::HostData,
    shared_data: sync::DArc<rt::SharedData>,
    message: &MessageData,
) -> Result<Arc<rt::supervisor::Supervisor>> {
    let metrics = shared_data.gep(|x| &x.metrics);

    let modules = modules::All {
        web: Arc::new(modules::Module::new(
            "web".into(),
            config.modules.web.address.clone(),
            shared_data.cancellation.clone(),
            shared_data.cookie.clone(),
            host_data.clone(),
            metrics.gep(|x| &x.web_module),
        )),
        llm: Arc::new(modules::Module::new(
            "llm".into(),
            config.modules.llm.address.clone(),
            shared_data.cancellation.clone(),
            shared_data.cookie.clone(),
            host_data,
            metrics.gep(|x| &x.llm_module),
        )),
    };

    let limiter_det = rt::memlimiter::Limiter::new("det");

    let locked_slots = host.get_locked_slots_for_sender(
        calldata::Address::from(message.contract_address.raw()),
        calldata::Address::from(message.sender_address.raw()),
        &limiter_det,
    )?;

    let ctor = rt::supervisor::Ctor {
        shared_data,
        modules,
        limiter: rt::DetNondet {
            det: limiter_det,
            non_det: rt::memlimiter::Limiter::new("nondet"),
        },
        locked_slots,
    };

    rt::supervisor::Supervisor::start(config, ctor, host)
}

pub async fn run_with_impl(
    entry_message: MessageData,
    supervisor: Arc<rt::supervisor::Supervisor>,
    permissions: &str,
) -> anyhow::Result<rt::vm::FullRunOk> {
    let mut host = supervisor.host.lock().await;

    let mut entrypoint = Vec::new();
    host.get_calldata(&mut entrypoint)?;

    std::mem::drop(host);

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
            contract_address: calldata::Address::from(entry_message.contract_address.raw()),
            sender_address: calldata::Address::from(entry_message.sender_address.raw()),
            origin_address: calldata::Address::from(entry_message.origin_address.raw()),
            stack: Vec::new(),

            chain_id: num_bigint::BigInt::from_str(&entry_message.chain_id).unwrap(),
            value: entry_message.value.unwrap_or(0).into(),
            is_init: entry_message.is_init,
            datetime: entry_message.datetime,

            entry_kind: public_abi::EntryKind::Main,
            entry_data: entrypoint,
            entry_stage_data: calldata::Value::Null,
        },
        supervisor: supervisor.clone(),
        version: genvm_common::version::Version::ZERO,
        should_capture_fp: Arc::new(std::sync::atomic::AtomicBool::new(true)),
    };

    let vm = rt::supervisor::spawn(&supervisor, essential_data).await?;
    let vm = rt::supervisor::apply_contract_actions(&supervisor, vm).await?;
    vm.run().await
}

pub async fn run_with(
    entry_message: MessageData,
    supervisor: Arc<rt::supervisor::Supervisor>,
    permissions: &str,
) -> anyhow::Result<(rt::vm::RunOk, Option<rt::errors::Fingerprint>, Option<u32>)> {
    let res = run_with_impl(entry_message, supervisor.clone(), permissions).await;

    log_debug!("deterministic execution done");

    let res = match res {
        Ok(res) => Ok(res),
        Err(e) => rt::errors::unwrap_vm_errors_fingerprint(e).map(|(x, y)| (x, Some(y))),
    };

    let nondet_disagree_res = rt::supervisor::await_nondet_vms(&supervisor).await;

    log_debug!("non-deterministic execution done");

    let merged_result = match (res, nondet_disagree_res) {
        (Err(e_res), Err(e_nondet)) => {
            log_error!(error:ah = e_nondet; "non-deterministic execution failed");

            Err(e_res)
        }
        (Err(e_res), Ok(_)) => Err(e_res),
        (Ok(_), Err(e_nondet)) => Err(e_nondet),
        (Ok((a, b)), Ok(c)) => Ok((a, b, c)),
    };

    let res = if supervisor.shared_data.cancellation.is_cancelled() {
        match merged_result {
            Ok((rt::vm::RunOk::VMError(msg, cause), fp, disag)) => Ok((
                rt::vm::RunOk::VMError(
                    public_abi::VmError::Timeout.value().into(),
                    cause.map(|v| v.context(msg)),
                ),
                fp,
                disag,
            )),
            Ok(r) => Ok(r),
            Err(e) => Ok((
                rt::vm::RunOk::VMError(public_abi::VmError::Timeout.value().into(), Some(e)),
                None,
                None,
            )),
        }
    } else {
        merged_result
    };

    let res = res.inspect_err(|e| {
        log_error!(error:ah = &e; "internal error");
    });

    if let Ok((_, _, Some(disag))) = &res {
        let mut host = supervisor.host.lock().await;
        host.notify_nondet_disagreement(*disag)?;
    }

    log_debug!("all executions done, collecting stats");

    let web_metrics = supervisor
        .modules
        .web
        .get_stats(genvm_modules_interfaces::web::Message::GetStats)
        .await
        .ok();
    let llm_metrics = supervisor
        .modules
        .llm
        .get_stats(genvm_modules_interfaces::llm::Message::GetStats)
        .await
        .ok();

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

    log_debug!("sending final result to host");

    let (res, nondet_disagree) = match res {
        Ok((a, b, c)) => (Ok((a, b)), c),
        Err(e) => (Err(e), None),
    };

    let mut host = supervisor.host.lock().await;
    host.consume_result(&res)?;
    std::mem::drop(host);

    match res {
        Ok((a, b)) => Ok((a, b, nondet_disagree)),
        Err(e) => Err(e),
    }
}
