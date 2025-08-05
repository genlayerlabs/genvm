use genvm_common::*;
use std::sync::{atomic::AtomicU32, Arc};

use crate::{public_abi, rt};

#[derive(Clone)]
pub struct Limiter {
    id: &'static str,
    remaining_memory: Arc<AtomicU32>,
    least_remaining_memory: Arc<AtomicU32>,
}

pub struct SaveTok {
    remaining_memory: u32,
}

impl Limiter {
    pub fn get_least_remaining_memory(&self) -> u32 {
        self.least_remaining_memory
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn new(id: &'static str) -> Self {
        Self {
            id,
            remaining_memory: Arc::new(AtomicU32::new(u32::MAX)),
            least_remaining_memory: Arc::new(AtomicU32::new(u32::MAX)),
        }
    }

    pub fn save(&self) -> SaveTok {
        SaveTok {
            remaining_memory: self
                .remaining_memory
                .load(std::sync::atomic::Ordering::SeqCst),
        }
    }

    pub fn restore(&self, tok: SaveTok) {
        self.remaining_memory
            .store(tok.remaining_memory, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn consume_mul(&self, delta: u32, multiplier: u32) -> bool {
        let delta = match delta.checked_mul(multiplier) {
            Some(delta) => delta,
            None => return false,
        };

        self.consume(delta)
    }

    pub fn get_remaining_memory(&self) -> u32 {
        self.remaining_memory
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn release(&self, delta: u32) {
        self.remaining_memory
            .fetch_add(delta, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn consume(&self, delta: u32) -> bool {
        let mut remaining = self
            .remaining_memory
            .load(std::sync::atomic::Ordering::SeqCst);

        log_debug!(delta = delta, remaining_at_op_start = remaining, id = self.id; "consume");

        loop {
            if delta > remaining {
                return false;
            }

            match self.remaining_memory.compare_exchange(
                remaining,
                remaining - delta,
                std::sync::atomic::Ordering::SeqCst,
                std::sync::atomic::Ordering::SeqCst,
            ) {
                Ok(_) => {
                    let least_for_test = remaining - delta;
                    self.least_remaining_memory
                        .fetch_min(least_for_test, std::sync::atomic::Ordering::SeqCst);
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
