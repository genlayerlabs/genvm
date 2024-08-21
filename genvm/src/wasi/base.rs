#[derive(Clone)]
pub struct Config {
    pub is_deterministic: bool,
    pub can_read_storage: bool,
    pub can_write_storage: bool,
}
