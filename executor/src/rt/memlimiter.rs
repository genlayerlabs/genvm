use genvm_common::*;
use std::sync::{atomic::AtomicU32, Arc};

use crate::{public_abi, rt};

struct LimiterInnerData {
    remaining_memory: Arc<AtomicU32>,
    least_remaining_memory: Arc<AtomicU32>,
}

struct LimiterInner {
    data: Arc<LimiterInnerData>,
    id: &'static str,
    consumed_memory: AtomicU32,
}

#[derive(Clone)]
pub struct Limiter(Arc<LimiterInner>);

impl Drop for LimiterInner {
    fn drop(&mut self) {
        let consumed = self
            .consumed_memory
            .load(std::sync::atomic::Ordering::SeqCst);

        log_debug!(id = self.id, consumed = consumed; "limiter drop");

        self.release_no_consumed(consumed);
    }
}

impl LimiterInner {
    fn release_no_consumed(&self, delta: u32) {
        #[allow(dead_code)]
        let previous = self
            .data
            .remaining_memory
            .fetch_add(delta, std::sync::atomic::Ordering::SeqCst);
        assert!(previous.checked_add(delta).is_some());
    }
}

impl Limiter {
    pub fn get_least_remaining_memory(&self) -> u32 {
        self.0
            .data
            .least_remaining_memory
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn new(id: &'static str) -> Self {
        Self(Arc::new(LimiterInner {
            id,
            consumed_memory: AtomicU32::new(0),
            data: Arc::new(LimiterInnerData {
                remaining_memory: Arc::new(AtomicU32::new(u32::MAX)),
                least_remaining_memory: Arc::new(AtomicU32::new(u32::MAX)),
            }),
        }))
    }

    pub fn derived(&self) -> Self {
        Self(Arc::new(LimiterInner {
            id: self.0.id,
            consumed_memory: AtomicU32::new(0),
            data: Arc::new(LimiterInnerData {
                remaining_memory: self.0.data.remaining_memory.clone(),
                least_remaining_memory: self.0.data.least_remaining_memory.clone(),
            }),
        }))
    }

    pub fn consume_mul(&self, delta: u32, multiplier: u32) -> bool {
        let delta = match delta.checked_mul(multiplier) {
            Some(delta) => delta,
            None => return false,
        };

        self.consume(delta)
    }

    pub fn get_remaining_memory(&self) -> u32 {
        self.0
            .data
            .remaining_memory
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn release(&self, delta: u32) {
        self.0.release_no_consumed(delta);
        self.0
            .consumed_memory
            .fetch_sub(delta, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn consume(&self, delta: u32) -> bool {
        let mut remaining = self
            .0
            .data
            .remaining_memory
            .load(std::sync::atomic::Ordering::SeqCst);

        log_debug!(delta = delta, remaining_at_op_start = remaining, id = self.0.id; "consume");

        loop {
            if delta > remaining {
                return false;
            }

            match self.0.data.remaining_memory.compare_exchange(
                remaining,
                remaining - delta,
                std::sync::atomic::Ordering::SeqCst,
                std::sync::atomic::Ordering::SeqCst,
            ) {
                Ok(_) => {
                    let least_for_test = remaining - delta;
                    self.0
                        .data
                        .least_remaining_memory
                        .fetch_min(least_for_test, std::sync::atomic::Ordering::SeqCst);
                    self.0
                        .consumed_memory
                        .fetch_add(delta, std::sync::atomic::Ordering::SeqCst);
                    break;
                }
                Err(new_remaining) => remaining = new_remaining,
            }
        }

        true
    }
}

impl wasmtime::ResourceLimiter for Limiter {
    fn memory_growing(
        &mut self,
        current: usize,
        desired: usize,
        _maximum: Option<usize>,
    ) -> anyhow::Result<bool> {
        let delta = desired - current;
        if delta > u32::MAX as usize {
            return Ok(false);
        }

        let delta = delta as u32;
        let success = self.consume(delta);

        if current == 0 && !success {
            Err(rt::errors::VMError::oom(None).into())
        } else {
            Ok(success)
        }
    }

    fn table_growing(
        &mut self,
        current: usize,
        desired: usize,
        _maximum: Option<usize>,
    ) -> anyhow::Result<bool> {
        let delta = desired - current;

        if delta > u32::MAX as usize {
            return Ok(false);
        }

        let delta = delta as u32;
        let success = self.consume_mul(delta, public_abi::MemoryLimiterConsts::TableEntry.value());

        if current == 0 && !success {
            Err(rt::errors::VMError::oom(None).into())
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
