pub mod errors;
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

#[derive(Debug)]
pub struct DataFeesLimit {
    remaining: tokio::sync::Mutex<primitive_types::U256>,

    storage_page_cost: u32,
    receipt_word_cost: u32,
}

impl DataFeesLimit {
    pub fn new(
        full: primitive_types::U256,
        storage_page_cost: u32,
        receipt_word_cost: u32,
    ) -> Self {
        Self {
            remaining: tokio::sync::Mutex::new(full),
            storage_page_cost,
            receipt_word_cost,
        }
    }

    pub async fn consume_raw(&self, amount: u64) -> bool {
        let mut remaining = self.remaining.lock().await;
        let amount = primitive_types::U256::from(amount);
        if *remaining >= amount {
            *remaining -= amount;
            true
        } else {
            false
        }
    }

    pub async fn remaining(&self) -> primitive_types::U256 {
        *self.remaining.lock().await
    }

    pub async fn consume_storage_pages(&self, pages: u64) -> bool {
        let cost = match pages.checked_mul(self.storage_page_cost as u64) {
            Some(c) => c,
            None => return false,
        };
        self.consume_raw(cost).await
    }

    pub async fn consume_receipt_words(&self, words: u64) -> bool {
        let cost = match words.checked_mul(self.receipt_word_cost as u64) {
            Some(c) => c,
            None => return false,
        };
        self.consume_raw(cost).await
    }
}

/// basic data that is shared across all VMs
pub struct SharedData {
    pub cancellation: Arc<genvm_common::cancellation::Token>,
    pub is_sync: bool,
    pub genvm_id: genvm_modules_interfaces::GenVMId,
    pub debug_mode: bool,
    pub metrics: crate::Metrics,
    pub data_fees_limit: DataFeesLimit,
}

pub fn parse_host_data(
    zelf: &genvm_common::domain::ExecutionData,
) -> anyhow::Result<genvm_modules_interfaces::HostData> {
    serde_json::from_str(&zelf.host_data)
        .with_context(|| "parsing host_data from execution context")
}

pub async fn spawn_apply_run(
    supervisor: &Arc<supervisor::Supervisor>,
    vm: wasi::genlayer_sdk::SingleVMData,
) -> std::result::Result<vm::RunResult, anyhow::Error> {
    match spawn_apply_run_inner(supervisor, vm).await {
        Ok(res) => Ok(res),
        Err((e, vm_data)) => {
            match errors::unwrap_vm_errors_fingerprint(errors::UnwrapDynError::from(e)) {
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
    vm: wasi::genlayer_sdk::SingleVMData,
) -> std::result::Result<vm::RunResult, (anyhow::Error, wasi::genlayer_sdk::SingleVMData)> {
    let limiter = supervisor.limiter.get(vm.conf.is_deterministic).derived();

    let vm = supervisor::spawn(supervisor, vm, limiter).await?;

    let vm = supervisor::apply_contract_actions(supervisor, vm).await?;

    vm.run().await
}

use anyhow::Context;

use crate::wasi;
