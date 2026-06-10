pub mod errors;
pub mod fees;
pub mod memlimiter;
pub mod supervisor;
pub mod vm;

use std::sync::Arc;

#[derive(Default, Debug, serde::Serialize, genlayer_calldata::Encode)]
pub struct Metrics {
    precompile_hits: genvm_common::stats::metric::Count,
    compiled_modules: genvm_common::stats::metric::Count,
    compilation_time: genvm_common::stats::metric::Time,
}

pub struct DetNondet<T> {
    pub det: T,
    pub non_det: T,
}

impl<T> DetNondet<T> {
    pub fn get(&self, is_det: bool) -> &T {
        if is_det {
            &self.det
        } else {
            &self.non_det
        }
    }

    pub fn get_mut(&mut self, is_det: bool) -> &mut T {
        if is_det {
            &mut self.det
        } else {
            &mut self.non_det
        }
    }
}

/// basic data that is shared across all VMs
pub struct SharedData {
    pub cancellation: Arc<genvm_common::cancellation::Token>,
    pub is_sync: bool,
    pub genvm_id: genvm_modules_interfaces::GenVMId,
    pub debug_mode: bool,
    pub metrics: crate::Metrics,
    pub data_fees_limit: fees::DataLimit,
    pub llm_consumption: tokio::sync::Mutex<primitive_types::U256>,
}

pub fn parse_host_data(
    zelf: &genvm_common::domain::ExecutionData,
) -> anyhow::Result<genvm_modules_interfaces::HostData> {
    serde_json::from_str(&zelf.host_data)
        .with_context(|| "parsing host_data from execution context")
}

pub async fn spawn_apply_run(
    supervisor: &Arc<supervisor::Supervisor>,
    vm: Box<wasi::genlayer_sdk::SingleVMData>,
) -> std::result::Result<vm::RunResult, anyhow::Error> {
    match spawn_apply_run_inner(supervisor, vm).await {
        Ok(res) => Ok(res),
        Err((e, vm_data)) => {
            // The store has already been consumed by `vm.run()` by the time we
            // get here, so no memory fingerprint can be taken; only the
            // backtrace frames (carried by the error) are recovered.
            match errors::unwrap_vm_errors_fingerprint(
                errors::UnwrapDynError::from(e),
                Default::default(),
            ) {
                Ok((run_ok, fp)) => Ok(vm::RunResult {
                    run_ok,
                    fingerprint: Some(fp),
                    vm_data,
                }),
                Err(e) => Err(e),
            }
        }
    }
}

async fn spawn_apply_run_inner(
    supervisor: &Arc<supervisor::Supervisor>,
    vm: Box<wasi::genlayer_sdk::SingleVMData>,
) -> std::result::Result<vm::RunResult, (anyhow::Error, Box<wasi::genlayer_sdk::SingleVMData>)> {
    let limiter = supervisor.limiter.get(vm.conf.is_deterministic).derived();

    let vm = supervisor::spawn(supervisor, vm, limiter).await?;

    let vm = supervisor::apply_contract_actions(supervisor, vm).await?;

    vm.run().await
}

use anyhow::Context;

use crate::wasi;
