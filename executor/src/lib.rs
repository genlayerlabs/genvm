pub mod caching;
pub mod config;
pub mod errors;
mod host;
pub mod memlimiter;
pub mod mmap;
pub mod modules;
pub mod runner;
pub mod ustar;
pub mod vm;
pub mod wasi;

pub mod public_abi;

pub mod version_timestamps;

pub use genvm_common::calldata;
use genvm_common::*;

pub use host::{Host, MessageData, SlotID};

use anyhow::{Context, Result};
use wasi::genlayer_sdk::ExtendedMessage;

use std::{str::FromStr, sync::Arc};
use vm::{Modules, RunOk};

#[derive(Debug, Clone)]
pub struct PublicArgs<'a> {
    pub cookie: String,
    pub debug_mode: bool,
    pub is_sync: bool,
    pub message: &'a MessageData,
}

pub fn create_supervisor(
    config: &config::Config,
    mut host: Host,
    cancellation: Arc<genvm_common::cancellation::Token>,
    host_data: genvm_modules_interfaces::HostData,
    pub_args: PublicArgs,
) -> Result<Arc<tokio::sync::Mutex<vm::Supervisor>>> {
    let modules = Modules {
        web: Arc::new(modules::Module::new(
            "web".into(),
            config.modules.web.address.clone(),
            cancellation.clone(),
            pub_args.cookie.clone(),
            host_data.clone(),
        )),
        llm: Arc::new(modules::Module::new(
            "llm".into(),
            config.modules.llm.address.clone(),
            cancellation.clone(),
            pub_args.cookie.clone(),
            host_data,
        )),
    };

    let limiter_det = memlimiter::Limiter::new("det");

    let locked_slots = host.get_locked_slots_for_sender(
        calldata::Address::from(pub_args.message.contract_address.raw()),
        calldata::Address::from(pub_args.message.sender_address.raw()),
        &limiter_det,
    )?;

    let shared_data = Arc::new(crate::vm::SharedData::new(
        modules,
        cancellation,
        pub_args.is_sync,
        pub_args.cookie.clone(),
        pub_args.debug_mode,
        limiter_det,
        locked_slots,
    ));

    Ok(Arc::new(tokio::sync::Mutex::new(vm::Supervisor::new(
        config,
        host,
        shared_data,
    )?)))
}

pub async fn run_with_impl(
    entry_message: MessageData,
    supervisor: Arc<tokio::sync::Mutex<vm::Supervisor>>,
    permissions: &str,
) -> anyhow::Result<vm::FullRunOk> {
    let (mut vm, instance) = {
        let supervisor_clone = supervisor.clone();

        let mut supervisor = supervisor.lock().await;

        let mut entrypoint = Vec::new();
        supervisor.host.get_calldata(&mut entrypoint)?;

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
            supervisor: supervisor_clone,
            version: genvm_common::version::Version::ZERO,
            should_capture_fp: Arc::new(std::sync::atomic::AtomicBool::new(true)),
        };

        let mut vm = supervisor.spawn(essential_data).await?;
        let instance = supervisor
            .apply_contract_actions(&mut vm)
            .await
            .with_context(|| "applying runner actions")
            .map_err(|cause| crate::errors::VMError::wrap("runner_actions".into(), cause))?;
        (vm, instance)
    };

    vm.run(&instance).await
}

pub async fn run_with(
    entry_message: MessageData,
    supervisor: Arc<tokio::sync::Mutex<vm::Supervisor>>,
    permissions: &str,
) -> anyhow::Result<(RunOk, Option<errors::Fingerprint>, Option<u32>)> {
    let sup_lock = supervisor.lock().await;
    let supervisor_data = sup_lock.supervisor_shared_data.clone();
    std::mem::drop(sup_lock);

    let res = run_with_impl(entry_message, supervisor.clone(), permissions).await;

    log_debug!("deterministic execution done");

    let res = match res {
        Ok(res) => Ok(res),
        Err(e) => errors::unwrap_vm_errors_fingerprint(e).map(|(x, y)| (x, Some(y))),
    };

    let nondet_disagree_res = supervisor_data.finish().await;

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

    let res = if supervisor_data.shared_data.cancellation.is_cancelled() {
        match merged_result {
            Ok((RunOk::VMError(msg, cause), fp, disag)) => Ok((
                RunOk::VMError(
                    public_abi::VmError::Timeout.value().into(),
                    cause.map(|v| v.context(msg)),
                ),
                fp,
                disag,
            )),
            Ok(r) => Ok(r),
            Err(e) => Ok((
                RunOk::VMError(public_abi::VmError::Timeout.value().into(), Some(e)),
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

    let mut supervisor = supervisor.lock().await;

    if let Ok((_, _, Some(disag))) = &res {
        supervisor.host.notify_nondet_disagreement(*disag)?;
    }

    supervisor.log_stats();

    log_debug!("sending final result to host");

    let (res, nondet_disagree) = match res {
        Ok((a, b, c)) => (Ok((a, b)), c),
        Err(e) => (Err(e), None),
    };

    supervisor.host.consume_result(&res)?;

    match res {
        Ok((a, b)) => Ok((a, b, nondet_disagree)),
        Err(e) => Err(e),
    }
}
