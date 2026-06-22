use serde::Serialize;

use crate::public_abi;

#[derive(Clone, Copy, Serialize, genlayer_calldata::Encode)]
pub struct Config {
    pub needs_error_fingerprint: bool,
    pub is_deterministic: bool,
    pub can_read_storage: bool,
    pub can_write_storage: bool,
    pub can_spawn_nondet: bool,
    pub can_send_messages: bool,
    pub can_call_others: bool,
    pub can_register_runners: bool,
    pub state_mode: public_abi::StorageType,
    pub code_slot: crate::SlotID,
}

impl Config {
    pub fn is_main(&self) -> bool {
        self.state_mode == public_abi::StorageType::Default
    }
}
