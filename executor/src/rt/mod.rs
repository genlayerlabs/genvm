pub mod errors;
pub mod memlimiter;
pub mod supervisor;
pub mod vm;

use std::sync::Arc;

#[derive(Default, Debug, serde::Serialize)]
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
    pub cookie: String,
    pub debug_mode: bool,
    pub metrics: crate::Metrics,
}
