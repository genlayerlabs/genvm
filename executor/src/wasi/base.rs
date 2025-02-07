use serde::Serialize;

#[derive(Clone, Copy, Serialize)]
pub struct Config {
    pub is_deterministic: bool,
    pub can_read_storage: bool,
    pub can_write_storage: bool,
    pub can_spawn_nondet: bool,
    pub can_send_messages: bool,
    pub can_call_others: bool,
    pub state_mode: crate::host::StorageType,
}

impl Config {
    pub fn is_main(&self) -> bool {
        self.state_mode == crate::host::StorageType::Default
    }
}
