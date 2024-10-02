use serde::Serialize;

#[derive(Clone, Copy, Serialize)]
pub struct Config {
    pub is_deterministic: bool,
    pub can_read_storage: bool,
    pub can_write_storage: bool,
    pub can_spawn_nondet: bool,
}
