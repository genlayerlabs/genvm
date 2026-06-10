pub mod errors;
pub mod fees;
pub mod memlimiter;
pub mod supervisor;
pub mod vm;

use std::sync::Arc;

enum SpawnErrorState {
    Spawned(vm::VMBase),
    Unspawned(Box<wasi::genlayer_sdk::SingleVMData>),
}

pub struct SpawnError {
    error: anyhow::Error,
    state: Box<SpawnErrorState>,
}

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
        Err(SpawnError { error, state }) => {
            let (memory_hashes, vm_data) = match *state {
                SpawnErrorState::Spawned(mut vm_base) => (
                    vm_base.memory_hashes(),
                    Box::new(vm_base.store.into_data().genlayer_ctx.genlayer_sdk.data),
                ),
                SpawnErrorState::Unspawned(vm_data) => (Default::default(), vm_data),
            };
            match errors::unwrap_vm_errors_backtrace(errors::UnwrapDynError::from(error)) {
                Ok((run_ok, backtrace)) => Ok(vm::RunResult {
                    run_ok,
                    backtrace,
                    memory_hashes,
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
) -> std::result::Result<vm::RunResult, SpawnError> {
    let limiter = supervisor.limiter.get(vm.conf.is_deterministic).derived();

    let vm = supervisor::spawn(supervisor, vm, limiter).await?;

    let vm = supervisor::apply_contract_actions(supervisor, vm).await?;

    vm.run().await
}

use anyhow::Context;

use crate::wasi;
